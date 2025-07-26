use pinocchio::{
    account_info::AccountInfo, 
    pubkey::Pubkey, 
    ProgramResult, 
    program_error::ProgramError,
    sysvars::{clock::Clock, Sysvar}
};

use pinocchio_log::log;

use crate::state::{multisig, proposal, Multisig, ProposalState, ProposalStatus};

pub fn init_proposal(
    multisig: &AccountInfo,
    proposal: &AccountInfo,
    proposer: &AccountInfo,
    proposal_expiry_duration: u64
) -> ProgramResult {

    log!("Initializing new proposal");

    let multisig_account = Multisig::from_account_info(multisig)?;
    log!("Multisig loaded with {} members", multisig_account.member_count);

    let proposer_pubkey = *proposer.key();
    let mut is_member = false;

    for i in 0..multisig_account.member_count as usize {
        let member_pubkey = Pubkey::from(multisig_account.memeber_keys[i]);
        if member_pubkey == proposer_pubkey {
            is_member = true;
            break;
        }
    }

    if !is_member {
        log!("Proposer {} is not a member of the multisig", &proposer_pubkey);
        return Err(ProgramError::InvalidAccountData);
    }

    log!("Verified proposer {} is a multisig member", &proposer_pubkey);

    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp as u64;
    let expiry_time = current_time + proposal_expiry_duration;

    log!("Current time: {}, Proposal will expire at: {}", current_time, expiry_time);

    let proposal_account = ProposalState::from_account_info(proposal)?;

    let proposal_id = multisig_account.total_proposals;
    multisig_account.total_proposals += 1;

    log!("Creating proposal with ID: {}", proposal_id);
    
    // Copy active members from multisig to proposal
    let mut active_members = [Pubkey::default(); 10];
    for i in 0..multisig_account.member_count as usize {
        active_members[i] = Pubkey::from(multisig_account.memeber_keys[i]);
    }
    
    // Initialize votes array (0 = NOT_VOTED for all members)
    let votes = [0u8; 10];

    proposal_account.proposal_id = proposal_id;
    proposal_account.expiry = expiry_time;
    proposal_account.result = ProposalStatus::Active;
    proposal_account.bump = 0; // You'll need to pass this from the PDA derivation
    proposal_account.active_members = active_members.map(|pk| pk);
    proposal_account.votes = votes;
    proposal_account.created_time = current_time;
    
    log!("Proposal initialized successfully!");
    log!("  - Proposal ID: {}", proposal_id);
    log!("  - Status: Active");
    log!("  - Active members: {}", multisig_account.member_count);
    log!("  - Expires at: {}", expiry_time);

    Ok(())
}

// -------------------------- TESTING init_proposal -----------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use mollusk_svm::{program, Mollusk, result::Check};
    use solana_sdk::{
        account::Account,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        pubkey,
        sysvar,
        clock::Clock,
    };
    use crate::state::{Multisig, ProposalState, ProposalStatus};

    const PROGRAM_ID: Pubkey = pubkey!("3X4xfxBGSWDc24HhACGxk5VdDAJzg9mxtUvvHvwjQcec");
    const CREATOR: Pubkey = Pubkey::new_from_array([1u8; 32]);

    #[test]
    fn test_init_proposal() {
        println!("Starting init proposal test");

        let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/multisig_pinocchio");
        println!("Mollusk initialized with program ID: {}", PROGRAM_ID);

        // Create multisig PDA
        let (multisig_pda, bump1) = Pubkey::find_program_address(
            &[b"multisig", CREATOR.as_ref()],
            &PROGRAM_ID
        );
        println!("Multisig PDA: {} (bump: {})", multisig_pda, bump1);

        // Create proposal PDA
        let proposal_id = 0u64;
        let (proposal_pda, bump2) = Pubkey::find_program_address(
            &[b"proposal", multisig_pda.as_ref(), &proposal_id.to_le_bytes()],
            &PROGRAM_ID
        );
        println!("Proposal PDA: {} (bump: {})", proposal_pda, bump2);

        // Define multisig members
        let member_1 = Pubkey::new_from_array([10u8; 32]);
        let member_2 = Pubkey::new_from_array([20u8; 32]);
        let member_3 = Pubkey::new_from_array([30u8; 32]);
        let proposer = member_1; // Use member_1 as proposer

        let mut initial_members = [Pubkey::default(); 10];
        initial_members[0] = member_1;
        initial_members[1] = member_2;
        initial_members[2] = member_3;

        println!("Initial members:");
        println!("  Member 1 (proposer): {}", member_1);
        println!("  Member 2: {}", member_2);
        println!("  Member 3: {}", member_3);

        // Create multisig account data
        let mut multisig_account_data = vec![0u8; Multisig::LEN];
        let mut member_keys = [[0u8; 32]; 10];
        for (i, pk) in initial_members.iter().enumerate() {
            member_keys[i] = pk.to_bytes();
        }

        let multisig = Multisig {
            creator: CREATOR.to_bytes(),
            member_count: 3,
            memeber_keys: member_keys,
            threshold: 2,
            proposal_expiry: 86400,
            total_proposals: 0,
            treasury_wallet: Pubkey::new_from_array([99u8; 32]).to_bytes(),
            config_bump: 255,
            treasury_bump: 254
        };

        println!("Created multisig with {} members, threshold: {}", multisig.member_count, multisig.threshold);

        unsafe {
            std::ptr::copy_nonoverlapping(
                &multisig as *const Multisig as *const u8,
                multisig_account_data.as_mut_ptr(),
                Multisig::LEN,
            );
        }

        let multisig_account = Account {
            lamports: 1_000_000,
            data: multisig_account_data,
            owner: PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        };

        // Create empty proposal account
        let proposal_account_data = vec![0u8; ProposalState::LEN];
        let proposal_account = Account {
            lamports: 1_000_000,
            data: proposal_account_data,
            owner: PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        };

        // Create proposer account (member account)
        let proposer_account = Account {
            lamports: 1_000_000,
            data: vec![],
            owner: solana_sdk::system_program::id(),
            executable: false,
            rent_epoch: 0,
        };

        // Create clock sysvar account
        let current_time = 1640995200i64; // Jan 1, 2022
        let clock = Clock {
            slot: 1000,
            epoch_start_timestamp: current_time - 3600,
            epoch: 10,
            leader_schedule_epoch: 10,
            unix_timestamp: current_time,
        };

        let clock_data = unsafe {
            std::slice::from_raw_parts(
                &clock as *const Clock as *const u8,
                std::mem::size_of::<Clock>(),
            ).to_vec()
        };
        
        let clock_account = Account {
            lamports: 1,
            data: clock_data,
            owner: sysvar::id(),
            executable: false,
            rent_epoch: 0,
        };

        println!("Clock sysvar created with timestamp: {}", current_time);

        // Create instruction data
        let proposal_expiry_duration = 86400u64; // 24 hours
        let mut instruction_data = vec![0u8; 9];
        instruction_data[0] = 4; // init_proposal discriminator (adjust based on your program)
        instruction_data[1..9].copy_from_slice(&proposal_expiry_duration.to_le_bytes());

        println!("Instruction data created:");
        println!("  Discriminator: {}", instruction_data[0]);
        println!("  Proposal expiry duration: {} seconds", proposal_expiry_duration);

        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(multisig_pda, false),
                AccountMeta::new(proposal_pda, false),
                AccountMeta::new_readonly(proposer, true), // Proposer must sign
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data: instruction_data,
        };

        println!("Executing init_proposal instruction...");

        // Test successful proposal creation
        mollusk.process_and_validate_instruction(
            &instruction,
            &vec![
                (multisig_pda, multisig_account.clone()),
                (proposal_pda, proposal_account.clone()),
                (proposer, proposer_account.clone()),
                (sysvar::clock::id(), clock_account.clone()),
            ],
            &[Check::success()],
        );

        println!("Instruction executed successfully!");
        println!("Proposal should now be initialized");
        println!("Proposal ID: {}", proposal_id);
        println!("Proposer: {}", proposer);
        println!("Expiry duration: {} seconds", proposal_expiry_duration);
    }
}