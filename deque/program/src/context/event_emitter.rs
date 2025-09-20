use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
};

use crate::validation::{event_authority::EventAuthorityInfo, self_program::SelfProgramInfo};

pub(crate) struct EventEmitterContext<'a, 'info> {
    pub self_program: SelfProgramInfo<'a, 'info>,
    pub event_authority: EventAuthorityInfo<'a, 'info>,
}

impl<'a, 'info> EventEmitterContext<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        Ok(EventEmitterContext {
            self_program: SelfProgramInfo::new_checked(next_account_info(accounts_iter)?)?,
            event_authority: EventAuthorityInfo::new_checked(next_account_info(accounts_iter)?)?,
        })
    }
}
