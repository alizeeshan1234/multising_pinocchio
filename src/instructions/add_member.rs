use pinocchio::{
    account_info::AccountInfo, pubkey::Pubkey, ProgramResult, program_error::ProgramError
};

use pinocchio_log::log;

use crate::state::Multisig;

pub fn add_member(multisig: &AccountInfo, new_member: Pubkey) -> ProgramResult {

    let multisig_account = Multisig::from_account_info(multisig)?;

    if multisig_account.member_count >= 10 {
        log!("Cannot add member: maximum capacity (10) reached");
        return Err(ProgramError::InvalidAccountData);
    }

    for i in 0..multisig_account.member_count as usize {
        if multisig_account.memeber_keys[i] == new_member {
            log!("Member already exist in the member list!");
            return Err(ProgramError::InvalidAccountData);
        };
    };

    let member_index = multisig_account.member_count as usize;
    multisig_account.memeber_keys[member_index] = new_member;

    multisig_account.member_count += 1;

    log!("Successfully added member!");
    log!("New member count: {}", multisig_account.member_count);

    Ok(())
}

// -------------------------- TESTING add_member -----------------------------

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
    fn test_add_member () {
        println!("Starting add member test");

        let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/multisig_pinocchio");
        println!("Mollusk initialized with program ID: {}", PROGRAM_ID);

        let (multisig_pda, bump1) = Pubkey::find_program_address(
            &[b"multisig", CREATOR.as_ref()],
            &PROGRAM_ID
        );
        println!("Multisig PDA: {} (bump: {})", multisig_pda, bump1);

        let mut account_data = vec![0u8; Multisig::LEN];
        let mut initial_members = [Pubkey::default(); 10];
        initial_members[0] = Pubkey::new_from_array([10u8; 32]);
        initial_members[1] = Pubkey::new_from_array([20u8; 32]);

        println!("Initial members:");
        println!("Member 0: {}", initial_members[0]);
        println!("Member 1: {}", initial_members[1]);


        let mut member_keys = [[0u8; 32]; 10];
        for (i, pk) in initial_members.iter().enumerate() {
            member_keys[i] = pk.to_bytes();
        }

        let multisig = Multisig {
            creator: CREATOR.to_bytes(),
            member_count: 2,
            memeber_keys: member_keys,
            threshold: 2,
            proposal_expiry: 86400,
            total_proposals: 0,
            treasury_wallet: Pubkey::new_from_array([99u8; 32]).to_bytes(),
            config_bump: 255,
            treasury_bump: 254
        };

        println!("Created multisig with {} members", multisig.member_count);

        unsafe {
            std::ptr::copy_nonoverlapping(
                &multisig as *const Multisig as *const u8,
                account_data.as_mut_ptr(),
                Multisig::LEN,
            );
        };

        let multisig_account = Account {
            lamports: 1_000_000,
            data: account_data,
            owner: PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        };

        println!("Multisig account created with {} lamports", multisig_account.lamports);

        let new_member = Pubkey::new_from_array([30u8; 32]);
        println!("Attempting to add new member: {}", new_member);


        let mut instruction_data = vec![0u8; 33];
        instruction_data[0] = 2;
        instruction_data[1..33].copy_from_slice(&new_member.to_bytes());

        println!("Instruction data created:");
        println!("Discriminator: {}", instruction_data[0]);
        println!("New member bytes: {:?}", &instruction_data[1..33]);

        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![AccountMeta::new(multisig_pda, false)],
            data: instruction_data
        };

        println!("Executing add_member instruction...");

        mollusk.process_and_validate_instruction(
            &instruction,
            &vec![(multisig_pda, multisig_account)],
            &[Check::success()]
        );

        println!("Instruction executed successfully!");
        println!("Members: {:?}", multisig.memeber_keys);
        println!("Members: {}", multisig.member_count);
        println!("Add member test completed successfully!");

    }
}