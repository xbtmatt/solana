use solana_program::{account_info::AccountInfo, program_error::ProgramError, system_program};

use crate::require;

/// Represents the system program account.
#[derive(Clone)]
pub struct SystemProgramInfo<'a, 'info> {
    pub info: &'a AccountInfo<'info>,
}

impl<'a, 'info> SystemProgramInfo<'a, 'info> {
    pub fn new_checked(
        info: &'a AccountInfo<'info>,
    ) -> Result<SystemProgramInfo<'a, 'info>, ProgramError> {
        require!(
            info.key.as_ref() == system_program::id().as_ref(),
            ProgramError::IncorrectProgramId,
            "Invalid system program ID"
        )?;
        Ok(SystemProgramInfo { info })
    }
}
