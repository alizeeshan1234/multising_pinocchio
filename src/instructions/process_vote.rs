use pinocchio::{
    account_info::AccountInfo, 
    pubkey::Pubkey, 
    ProgramResult, 
    program_error::ProgramError,
    sysvars::{clock::Clock, Sysvar}
};

use pinocchio_log::log;

use crate::state::{multisig::Multisig, proposal::ProposalState, proposal::ProposalStatus, vote::VoteState};

// Vote types
pub const VOTE_NOT_VOTED: u8 = 0;
pub const VOTE_FOR: u8 = 1;
pub const VOTE_AGAINST: u8 = 2;
pub const VOTE_ABSTAIN: u8 = 3;

pub fn process_vote(
    multisig: &AccountInfo,
    proposal: &AccountInfo,
    voter: &AccountInfo,
    vote_account: &AccountInfo,
    vote_type: u8,
) -> ProgramResult {
    log!("Processing vote");

    // Validate vote type
    if vote_type > VOTE_ABSTAIN {
        log!("Invalid vote type: {}", vote_type);
        return Err(ProgramError::InvalidInstructionData);
    }

    // Load multisig account
    let multisig_account = Multisig::from_account_info(multisig)?;
    log!("Multisig loaded with {} members", multisig_account.member_count);

    // Load proposal account
    let mut proposal_account = ProposalState::from_account_info(proposal)?;
    log!("Proposal loaded with ID: {}", proposal_account.proposal_id);

    // Check if proposal is active
    if !matches!(proposal_account.result, ProposalStatus::Active) {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if proposal has expired
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp as u64;
    
    if current_time > proposal_account.expiry {
        log!("Proposal has expired. Current time: {}, Expiry: {}", current_time, proposal_account.expiry);
        // Mark proposal as failed due to expiry
        proposal_account.result = ProposalStatus::Failed;
        return Err(ProgramError::InvalidAccountData);
    }

    log!("Proposal is active and not expired");

    // Verify voter is a member and get their index
    let voter_pubkey = *voter.key();
    let mut voter_index: Option<usize> = None;

    for i in 0..multisig_account.member_count as usize {
        if multisig_account.memeber_keys[i] == voter_pubkey {
            voter_index = Some(i);
            break;
        }
    }

    let voter_idx = match voter_index {
        Some(idx) => idx,
        None => {
            return Err(ProgramError::InvalidAccountData);
        }
    };

    // Check if voter has already voted (and if they're changing their vote)
    let previous_vote = proposal_account.votes[voter_idx];
    if previous_vote != VOTE_NOT_VOTED {
        log!("Member has already voted. Previous vote: {}, New vote: {}", previous_vote, vote_type);
        // Allow vote changes - this is a design decision
        // You could return an error here if you don't want to allow vote changes
    }

    // Record the vote
    proposal_account.votes[voter_idx] = vote_type;
    log!("Vote recorded: Member {} voted {}", voter_idx, vote_type);

    // Update vote account if provided
    if vote_account.data_len() > 0 {
        let mut vote_state = VoteState::from_account_info(vote_account)?;
        vote_state.has_permission = true;
        vote_state.vote_count += 1;
        log!("Updated vote account - vote count: {}", vote_state.vote_count);
    }

    // Count current votes
    let mut votes_for = 0u64;
    let mut votes_against = 0u64;
    let mut votes_abstain = 0u64;
    let mut total_votes = 0u64;

    for i in 0..multisig_account.member_count as usize {
        match proposal_account.votes[i] {
            VOTE_FOR => {
                votes_for += 1;
                total_votes += 1;
            },
            VOTE_AGAINST => {
                votes_against += 1;
                total_votes += 1;
            },
            VOTE_ABSTAIN => {
                votes_abstain += 1;
                total_votes += 1;
            },
            VOTE_NOT_VOTED => {},
            _ => {
                log!("Invalid vote found at index {}: {}", i, proposal_account.votes[i]);
            }
        }
    }

    log!("Vote tally - For: {}, Against: {}, Abstain: {}, Total: {}", 
         votes_for, votes_against, votes_abstain, total_votes);

    // Check if proposal should be resolved
    let threshold = multisig_account.threshold;
    
    // Proposal succeeds if votes_for >= threshold
    if votes_for >= threshold {
        proposal_account.result = ProposalStatus::Succeeded;
        log!("Proposal succeeded! Votes for ({}) >= threshold ({})", votes_for, threshold);
    }
    // Proposal fails if it's impossible to reach threshold
    // (votes_against + remaining_votes < threshold needed)
    else if votes_against > multisig_account.member_count - threshold {
        proposal_account.result = ProposalStatus::Failed;
        log!("Proposal failed! Too many against votes to reach threshold");
    }
    // Proposal fails if all members have voted but threshold not met
    else if total_votes == multisig_account.member_count && votes_for < threshold {
        proposal_account.result = ProposalStatus::Failed;
        log!("Proposal failed! All members voted but threshold not reached");
    }
    // Otherwise, proposal remains active
    else {
        log!("Proposal remains active. Need {} more 'for' votes to reach threshold", 
             threshold.saturating_sub(votes_for));
    }

    log!("Vote processing completed successfully");

    Ok(())
}

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
    use pinocchio::program_error::ProgramError;
    use crate::state::{Multisig, ProposalState, ProposalStatus, VoteState};

    const PROGRAM_ID: Pubkey = pubkey!("3X4xfxBGSWDc24HhACGxk5VdDAJzg9mxtUvvHvwjQcec");
    const CREATOR: Pubkey = Pubkey::new_from_array([1u8; 32]);

    #[test]
    fn test_process_vote() {
        println!("Starting process_vote test");

        let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/multisig_pinocchio");
        println!("Mollusk initialized");

        // Create PDAs
        let (multisig_pda, _) = Pubkey::find_program_address(
            &[b"multisig", CREATOR.as_ref()],
            &PROGRAM_ID
        );

        let proposal_id = 0u64;
        let (proposal_pda, _) = Pubkey::find_program_address(
            &[b"proposal", multisig_pda.as_ref(), &proposal_id.to_le_bytes()],
            &PROGRAM_ID
        );

        // Define members
        let member_1 = Pubkey::new_from_array([10u8; 32]);
        let member_2 = Pubkey::new_from_array([20u8; 32]);
        let member_3 = Pubkey::new_from_array([30u8; 32]);

        let mut member_keys = [[0u8; 32]; 10];
        member_keys[0] = member_1.to_bytes();
        member_keys[1] = member_2.to_bytes();
        member_keys[2] = member_3.to_bytes();

        println!("Members: {}, {}, {}", member_1, member_2, member_3);

        // Create multisig account
        let mut multisig_data = vec![0u8; Multisig::LEN];
        let multisig = Multisig {
            creator: CREATOR.to_bytes(),
            member_count: 3,
            memeber_keys: member_keys,
            threshold: 2, // Need 2 votes to pass
            proposal_expiry: 86400,
            total_proposals: 1,
            treasury_wallet: Pubkey::new_from_array([99u8; 32]).to_bytes(),
            config_bump: 255,
            treasury_bump: 254
        };

        unsafe {
            std::ptr::copy_nonoverlapping(
                &multisig as *const Multisig as *const u8,
                multisig_data.as_mut_ptr(),
                Multisig::LEN,
            );
        }

        let multisig_account = Account {
            lamports: 1_000_000,
            data: multisig_data,
            owner: PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        };

        // Create active proposal account
        let current_time = 1640995200u64;
        let mut proposal_data = vec![0u8; ProposalState::LEN];
        
        let mut active_members = [[0u8; 32]; 10];
        active_members[0] = member_1.to_bytes();
        active_members[1] = member_2.to_bytes();
        active_members[2] = member_3.to_bytes();
        
        let proposal = ProposalState {
            proposal_id: 0,
            expiry: current_time + 86400,
            result: ProposalStatus::Active,
            bump: 255,
            active_members: active_members,
            votes: [0u8; 10], // All NOT_VOTED
            created_time: current_time,
        };

        unsafe {
            std::ptr::copy_nonoverlapping(
                &proposal as *const ProposalState as *const u8,
                proposal_data.as_mut_ptr(),
                ProposalState::LEN,
            );
        }

        let proposal_account = Account {
            lamports: 1_000_000,
            data: proposal_data,
            owner: PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        };

        // Create vote account
        let vote_pda = Pubkey::new_unique();
        let mut vote_data = vec![0u8; VoteState::LEN];
        let vote_state = VoteState {
            has_permission: true,
            vote_count: 0,
            bump: 255,
            votes: [0u8; 10],
        };

        unsafe {
            std::ptr::copy_nonoverlapping(
                &vote_state as *const VoteState as *const u8,
                vote_data.as_mut_ptr(),
                VoteState::LEN,
            );
        }

        let vote_account = Account {
            lamports: 1_000_000,
            data: vote_data,
            owner: PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        };

        // Create clock sysvar
        let clock = Clock {
            slot: 1000,
            epoch_start_timestamp: current_time as i64 - 3600,
            epoch: 10,
            leader_schedule_epoch: 10,
            unix_timestamp: current_time as i64,
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

        // Create member account
        let member_account = Account {
            lamports: 1_000_000,
            data: vec![],
            owner: solana_sdk::system_program::id(),
            executable: false,
            rent_epoch: 0,
        };

        // Test successful vote
        let instruction_data = vec![1u8]; // Just VOTE_FOR without discriminator

        let instruction = Instruction {
            program_id: PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(multisig_pda, false),
                AccountMeta::new(proposal_pda, false),
                AccountMeta::new_readonly(member_1, true), // Voter signs
                AccountMeta::new(vote_pda, false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
            ],
            data: instruction_data,
        };

        println!("Executing vote instruction for member_1 voting FOR");

        mollusk.process_and_validate_instruction(
            &instruction,
            &vec![
                (multisig_pda, multisig_account.clone()),
                (proposal_pda, proposal_account.clone()),
                (member_1, member_account.clone()),
                (vote_pda, vote_account.clone()),
                (sysvar::clock::id(), clock_account.clone()),
            ],
            &[Check::success()],
        );

        println!("✅ Vote recorded successfully!");
    }
}