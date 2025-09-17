use solana_program::{account_info::AccountInfo, program_error::ProgramError, system_program};

use crate::require;

/// Represents a completely uninitialized account.
#[derive(Clone)]
pub struct UninitializedAccountInfo<'a, 'info> {
    pub info: &'a AccountInfo<'info>,
}

impl<'a, 'info> UninitializedAccountInfo<'a, 'info> {
    pub fn new_checked(
        info: &'a AccountInfo<'info>,
    ) -> Result<UninitializedAccountInfo<'a, 'info>, ProgramError> {
        require!(
            info.data_is_empty(),
            ProgramError::InvalidAccountData,
            "Account must be uninitialized"
        )?;
        require!(
            info.owner.as_ref() == system_program::id().as_ref(),
            ProgramError::InvalidAccountOwner,
            "Uninitialized accounts must be owned by the system program"
        )?;
        Ok(UninitializedAccountInfo { info })
    }
}
