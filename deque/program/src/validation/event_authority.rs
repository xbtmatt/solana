use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{require, seeds};

#[derive(Clone)]
pub(crate) struct EventAuthorityInfo<'a, 'info> {
    pub info: &'a AccountInfo<'info>,
}

impl<'a, 'info> EventAuthorityInfo<'a, 'info> {
    pub fn new_checked(
        info: &'a AccountInfo<'info>,
    ) -> Result<EventAuthorityInfo<'a, 'info>, ProgramError> {
        require!(
            info.key.as_ref() == seeds::event_authority::ID.as_ref(),
            ProgramError::IncorrectAuthority,
            "Invalid event authority"
        )?;

        Ok(EventAuthorityInfo { info })
    }
}
