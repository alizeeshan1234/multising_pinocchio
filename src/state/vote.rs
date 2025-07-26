use pinocchio::{
    account_info::AccountInfo, 
    pubkey::Pubkey
};

#[repr(C)]
pub struct VoteState {
    pub has_permission: bool, // Indicates if the account has permission to vote
    pub vote_count: u64, // proposal counter
    pub bump: u8, // Bump seed for PDA   
    pub votes: [u8; 10], // Array to hold votes, adjust size as needed
}

impl VoteState {
    pub const LEN: usize = 1 + 8 + 1; // 1 byte for has_permission, 8 bytes for vote_count, and 1 byte for bump

    pub fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        unsafe { &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self) }
    }

    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, pinocchio::program_error::ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
        Ok(Self::from_account_info_unchecked(account_info))
    }
}