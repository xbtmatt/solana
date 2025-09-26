use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
};

use crate::validation::{event_authority::EventAuthorityInfo, system_program::SystemProgramInfo};

#[derive(Clone)]
pub struct EventAuthorityContext<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub event_authority: EventAuthorityInfo<'a, 'info>,
    pub system_program: SystemProgramInfo<'a, 'info>,
}

impl<'a, 'info> EventAuthorityContext<'a, 'info> {
    pub fn load(
        accounts: &'a [AccountInfo<'info>],
    ) -> Result<EventAuthorityContext<'a, 'info>, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let event_authority = EventAuthorityInfo::new_checked(next_account_info(accounts_iter)?)?;
        let system_program = SystemProgramInfo::new_checked(next_account_info(accounts_iter)?)?;

        Ok(EventAuthorityContext {
            payer,
            event_authority,
            system_program,
        })
    }
}
