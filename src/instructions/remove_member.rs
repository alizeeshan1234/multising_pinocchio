use pinocchio::{
    account_info::AccountInfo, pubkey::Pubkey, ProgramResult, program_error::ProgramError
};

use pinocchio_log::log;

use crate::state::Multisig;

pub fn remove_member(multisig: &AccountInfo, member_to_remove: Pubkey) -> ProgramResult {

    let multisig_account = Multisig::from_account_info(multisig)?;

    if multisig_account.member_count <= multisig_account.threshold {
        log!("Cannot remove member: would make threshold impossible to reach");
        return Err(ProgramError::InvalidAccountData);
    };

    let mut member_found = false;
    let mut member_index = 0;

    for i in 0..multisig_account.member_count as usize {
        if multisig_account.memeber_keys[i] == member_to_remove {
            member_found = true;
            member_index = i;
            break;
        }
    }

    if !member_found {
        log!("Member not found in multisig");
        return Err(ProgramError::InvalidAccountData);
    };

    // Shift all members after the removed one to fill the gap
    for i in member_index..(multisig_account.member_count as usize - 1) {
        multisig_account.memeber_keys[i] = multisig_account.memeber_keys[i + 1];
    }

    // Clear the last slot and decrement count
    multisig_account.memeber_keys[(multisig_account.member_count - 1) as usize] = Pubkey::default();
    multisig_account.member_count -= 1;
    
    log!("Successfully removed member!");
    log!("New member count: {}", multisig_account.member_count);


    Ok(())
}

// -------------------------- TESTING remove_member -----------------------------

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
    fn test_remove_member() {
        println!("Starting remove member test");

        let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/multisig_pinocchio");
        println!("Mollusk initialized with program ID: {}", PROGRAM_ID);

        let (multisig_pda, bump1) = Pubkey::find_program_address(
            &[b"multisig", CREATOR.as_ref()],
            &PROGRAM_ID
        );

        println!("Multisig PDA: {} (bump: {})", multisig_pda, bump1);

        let member_to_remove = Pubkey::new_from_array([10u8; 32]);
        let member_1 = Pubkey::new_from_array([20u8; 32]);
        let member_2 = Pubkey::new_from_array([30u8; 32]);
        
        let mut initial_members = [Pubkey::default(); 10];
        initial_members[0] = member_to_remove;
        initial_members[1] = member_1;
        initial_members[2] = member_2;

        println!("Initial members:");
        println!("  Member 0 (to remove): {}", member_to_remove);
        println!("  Member 1: {}", member_1);
        println!("  Member 2: {}", member_2);

        let mut account_data = vec![0u8; Multisig::LEN];
        let mut member_keys = [[0u8; 32]; 10];
        for (i, pk) in initial_members.iter().enumerate() {
            member_keys[i] = pk.to_bytes();
        };

        let multisig = Multisig {
            creator: CREATOR.to_bytes(),
            member_count: 3, // 3 members with threshold 2
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

        let mut instruction_data = vec![0u8; 33];
        instruction_data[0] = 3; // remove_member discriminator
        instruction_data[1..33].copy_from_slice(&member_to_remove.to_bytes());

        println!("Instruction data created:");
        println!("  Discriminator: {}", instruction_data[0]);
        println!("  Member to remove: {}", member_to_remove);

        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![AccountMeta::new(multisig_pda, false)],
            data: instruction_data,
        };

        println!("Executing remove_member instruction...");

        mollusk.process_and_validate_instruction(
            &instruction,
            &vec![(multisig_pda, multisig_account)],
            &[Check::success()],
        );

        println!("Instruction executed successfully!");
        println!("Member {} should now be removed from multisig", member_to_remove);
        println!("Members: {:?}", multisig.memeber_keys);
        println!("Members: {}", multisig.member_count);
        println!("Remove member test completed successfully!");

    }

}