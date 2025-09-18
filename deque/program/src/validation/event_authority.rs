use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{events::event_authority, require};

#[derive(Clone)]
pub(crate) struct EventAuthorityInfo<'a, 'info> {
    pub info: &'a AccountInfo<'info>,
}

impl<'a, 'info> EventAuthorityInfo<'a, 'info> {
    pub fn new_checked(
        info: &'a AccountInfo<'info>,
    ) -> Result<EventAuthorityInfo<'a, 'info>, ProgramError> {
        require!(
            info.key.as_ref() == event_authority::PDA.as_ref(),
            ProgramError::IncorrectAuthority,
            "Invalid event authority"
        )?;

        Ok(EventAuthorityInfo { info })
    }
}
