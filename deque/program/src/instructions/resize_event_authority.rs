use solana_program::{
    account_info::AccountInfo,
    entrypoint::{ProgramResult, MAX_PERMITTED_DATA_INCREASE},
    msg,
    pubkey::Pubkey,
};

use crate::{context::event_authority_ctx::EventAuthorityContext, utils::fund_then_resize};

/// This doesn't actually need to do anything- it merely flushes the passed instruction data.
pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Resize event authority!");
    let ctx = EventAuthorityContext::load(accounts)?;

    fund_then_resize(
        ctx.event_authority.info,
        ctx.payer,
        ctx.system_program.info,
        MAX_PERMITTED_DATA_INCREASE,
    )?;

    Ok(())
}
