use pinocchio::{
    account_info::AccountInfo,
    pubkey::Pubkey,
};

#[repr(C)]
pub struct Multisig {
    pub creator: Pubkey, // Address that created the multisig
    pub member_count: u64, // Total number of members
    pub memeber_keys: [Pubkey; 10], // List of member public keys (max 10)
    pub threshold: u64,   // Minimum approvals required
    pub proposal_expiry: u64,     // Max duration a proposal can remain active
    pub total_proposals: u64,     // Counter to track number of proposals
    pub treasury_wallet: Pubkey,    // PDA for the multisig treasury
    pub config_bump: u8,    // Bump for this multisig config PDA
    pub treasury_bump: u8, // Bump for the treasury PDA
}

impl Multisig {
    pub const LEN: usize = 32 + 8 + (32 * 10) + 8 + 8 + 8 + 32 + 1 + 1;

    pub fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        unsafe {
            &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self)
        }
    }

    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, pinocchio::program_error::ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
        Ok(Self::from_account_info_unchecked(account_info))
    }
}