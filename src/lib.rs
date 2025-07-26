use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    ProgramResult,
};

entrypoint!(process_instruction);

mod state;
mod instructions;

use state::*;
use instructions::*;

pinocchio_pubkey::declare_id!("3X4xfxBGSWDc24HhACGxk5VdDAJzg9mxtUvvHvwjQcec");

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    assert_eq!(program_id, &ID);

    let (discriminator, data) = instruction_data.split_first().ok_or(ProgramError::InvalidInstructionData)?;

    match *discriminator {
        0 => {
            initialize_multisig(accounts, data)
        }
        1 => {
            let member = read_pubkey(data)?;
            let multisig = &accounts[0];
            add_member(multisig, member)
        }
        2 => {
            let member = read_pubkey(data)?;
            let multisig = &accounts[0];
            remove_member(multisig, member)
        }
        3 => {
            let proposal_expiry_duration = read_u64(&data[96..])?;
            let multisig = &accounts[0];
            let proposal = &accounts[1]; 
            let proposer = &accounts[2];
            init_proposal(multisig, proposal, proposer, proposal_expiry_duration)
        }
        4 => {
            let vote_type = data[128];
            let multisig = &accounts[0];
            let proposal = &accounts[1];
            let voter = &accounts[2];
            let vote_account = &accounts[3];
            process_vote(multisig, proposal, voter, vote_account, vote_type)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn read_pubkey(data: &[u8]) -> Result<Pubkey, ProgramError> {
    if data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&data[..32]);
    Ok(Pubkey::from(key_bytes))
}

fn read_u64(data: &[u8]) -> Result<u64, ProgramError> {
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[..8]);
    Ok(u64::from_le_bytes(bytes))
}
