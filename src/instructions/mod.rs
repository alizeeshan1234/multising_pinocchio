use pinocchio::{program_error::ProgramError, pubkey::Pubkey};

pub mod init_multisig;
pub use init_multisig::*;

pub mod add_member;
pub use add_member::*;

pub mod remove_member;
pub use remove_member::*;

pub mod init_proposal;
pub use init_proposal::*;

pub mod process_vote;
pub use process_vote::*;

// pub enum MultisigInstructions {
//     InitializeMultisig = 0,
//     AddMember = 1,
//     RemoveMember = 2,
//     InitializeProposal = 3,
//     Vote = 4,
// }

// impl TryFrom<&u8> for MultisigInstructions {
//     type Error = ProgramError;

//     fn try_from(value: &u8) -> Result<Self, Self::Error> {
//         match value {
//             0 => Ok(MultisigInstructions::InitializeMultisig),
//             1 => Ok(MultisigInstructions::AddMember),
//             2 => Ok(MultisigInstructions::RemoveMember),
//             3 => Ok(MultisigInstructions::InitializeProposal),
//             4 => Ok(MultisigInstructions::Vote),
//             _ => Err(ProgramError::InvalidInstructionData)
//         }
//     }
// }