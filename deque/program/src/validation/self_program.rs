use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{require, PROGRAM_ID_PUBKEY};

#[derive(Clone)]
pub struct SelfProgramInfo<'a, 'info> {
    pub info: &'a AccountInfo<'info>,
}

impl<'a, 'info> SelfProgramInfo<'a, 'info> {
    pub fn new_checked(
        info: &'a AccountInfo<'info>,
    ) -> Result<SelfProgramInfo<'a, 'info>, ProgramError> {
        require!(
            info.key.as_ref() == PROGRAM_ID_PUBKEY.as_ref(),
            ProgramError::IncorrectProgramId,
            "Invalid self program ID"
        )?;

        Ok(SelfProgramInfo { info })
    }
}
