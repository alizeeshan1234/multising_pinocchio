use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_log::log;

use pinocchio_system::instructions::CreateAccount;

use crate::state::Multisig;

pub fn initialize_multisig(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {

    let [creator, multisig, treasury_wallet, _remaining @..] = accounts else {
        return Err(ProgramError::InvalidAccountData)
    };

    // Check minimum data length: discriminator(1) + member_count(8) + threshold(1) = 10 bytes minimum
    if data.len() < 10 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let seed = [b"multisig", creator.key().as_ref()];
    let multisig_account_seeds = &seed[..];
    let (multisig_pda, multisig_bump) = pubkey::find_program_address(multisig_account_seeds, &crate::ID);

    if *multisig.key() != multisig_pda {
        return Err(ProgramError::InvalidSeeds);
    }

    let treasury_seed = [b"treasury", multisig.key().as_ref()];
    let treasury_seeds = &treasury_seed[..];
    let (treasury_pda, treasury_bump) = pubkey::find_program_address(treasury_seeds, &crate::ID);

    if *treasury_wallet.key() != treasury_pda {
        return Err(ProgramError::InvalidSeeds);
    }

    if *multisig.owner() == crate::ID && multisig.data_len() > 0 {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if *multisig.owner() != crate::ID {
        log!("Initializing Multisig Account");
        
        let lamports = Rent::get()?.minimum_balance(Multisig::LEN);

        CreateAccount {
            from: creator,
            to: multisig,
            lamports,
            space: Multisig::LEN as u64,
            owner: &crate::ID
        }.invoke()?;

        let multisig_account = Multisig::from_account_info(multisig)?;
        
        // Parse data safely - skip discriminator at data[0]
        let member_count = u64::from_le_bytes([
            data[1], data[2], data[3], data[4], 
            data[5], data[6], data[7], data[8]
        ]);

        let threshold = data[9] as u64;
        
        // Parse proposal_expiry - check if we have more data
        let proposal_expiry = if data.len() >= 18 {
            // If we have 8 more bytes, parse as u64
            u64::from_le_bytes([
                data[10], data[11], data[12], data[13],
                data[14], data[15], data[16], data[17]
            ])
        } else if data.len() > 10 {
            // If we have 1 more byte, parse as u8 and convert to u64
            data[10] as u64
        } else {
            // Default value
            86400 // 24 hours in seconds
        };

        // Validate parameters
        if member_count == 0 || member_count > 10 {
            return Err(ProgramError::InvalidInstructionData);
        };
        
        if threshold == 0 || threshold > member_count {
            return Err(ProgramError::InvalidInstructionData);
        };

        multisig_account.creator = *creator.key();
        multisig_account.member_count = member_count;
        multisig_account.memeber_keys = [Pubkey::default(); 10];
        multisig_account.threshold = threshold;
        multisig_account.proposal_expiry = proposal_expiry;
        multisig_account.total_proposals = 0;
        multisig_account.treasury_wallet = treasury_pda;
        multisig_account.config_bump = multisig_bump;
        multisig_account.treasury_bump = treasury_bump;

        log!("Multisig initialized successfully");
    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    Ok(())
}

// -------------------------- TESTING initialize_multisig -----------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use mollusk_svm::{program, Mollusk, result::Check};
    use solana_sdk::{
        account::Account,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        pubkey,
    };

    const PROGRAM_ID: Pubkey = pubkey!("3X4xfxBGSWDc24HhACGxk5VdDAJzg9mxtUvvHvwjQcec");
    const CREATOR: Pubkey = Pubkey::new_from_array([1u8; 32]);

    #[test]
    fn test_initialize_multisig_simple() {
        println!("Starting multisig initialization test");
        
        // Initialize mollusk with our program
        let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/multisig_pinocchio");
        println!("Mollusk initialized with program ID: {}", PROGRAM_ID);

        // Calculate PDAs
        let (multisig_pda, bump1) = Pubkey::find_program_address(
            &[b"multisig", CREATOR.as_ref()],
            &PROGRAM_ID
        );
        println!("Multisig PDA: {} (bump: {})", multisig_pda, bump1);

        let (treasury_pda, bump2) = Pubkey::find_program_address(
            &[b"treasury", multisig_pda.as_ref()],
            &PROGRAM_ID
        );
        println!("Treasury PDA: {} (bump: {})", treasury_pda, bump2);

        // Get system program
        let (system_program_id, system_account) = program::keyed_account_for_system_program();
        println!("System program ID: {}", system_program_id);

        // Create instruction data
        let mut instruction_data = vec![0u8; 10]; // Minimum required: discriminator + member_count + threshold
        instruction_data[0] = 1; // discriminator
        instruction_data[1..9].copy_from_slice(&3u64.to_le_bytes()); // member_count = 3
        instruction_data[9] = 2; // threshold = 2
        
        println!("Instruction data created:");
        println!("Discriminator: {}", instruction_data[0]);
        println!("Member count: {}", u64::from_le_bytes([instruction_data[1], instruction_data[2], instruction_data[3], instruction_data[4], instruction_data[5], instruction_data[6], instruction_data[7], instruction_data[8]]));
        println!("Threshold: {}", instruction_data[9]);
        println!("Total data length: {} bytes", instruction_data.len());

        // Create instruction
        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(CREATOR, true),
                AccountMeta::new(multisig_pda, false),
                AccountMeta::new_readonly(treasury_pda, false),
            ],
            data: instruction_data,
        };
        
        println!("Instruction created with {} accounts:", instruction.accounts.len());
        for (i, account_meta) in instruction.accounts.iter().enumerate() {
            println!("   {}. {} (writable: {}, signer: {})", 
                i, account_meta.pubkey, account_meta.is_writable, account_meta.is_signer);
        }

        // Create accounts
        let creator_account = Account {
            lamports: 10_000_000, // 0.01 SOL
            data: vec![],
            owner: solana_sdk::system_program::id(),
            executable: false,
            rent_epoch: 0,
        };
        println!("Creator account: {} lamports", creator_account.lamports);

        let multisig_account = Account {
            lamports: 0,
            data: vec![],
            owner: solana_sdk::system_program::id(),
            executable: false,
            rent_epoch: 0,
        };
        println!("Multisig account: {} lamports, {} bytes data", multisig_account.lamports, multisig_account.data.len());

        let treasury_account = Account {
            lamports: 0,
            data: vec![],
            owner: solana_sdk::system_program::id(),
            executable: false,
            rent_epoch: 0,
        };
        println!("Treasury account: {} lamports, {} bytes data", treasury_account.lamports, treasury_account.data.len());

        println!("Executing instruction...");
        
        // Execute and validate
        mollusk.process_and_validate_instruction(
            &instruction,
            &vec![
                (CREATOR, creator_account),
                (multisig_pda, multisig_account),
                (treasury_pda, treasury_account),
                (system_program_id, system_account),
            ],
            &[Check::success()],
        );
        
        println!("Test completed successfully!");
    }
}