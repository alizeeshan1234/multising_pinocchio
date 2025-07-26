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

