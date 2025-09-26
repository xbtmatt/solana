use solana_program::{
    account_info::AccountInfo,
    entrypoint::{ProgramResult, MAX_PERMITTED_DATA_INCREASE},
    msg,
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    context::event_authority_ctx::EventAuthorityContext, seeds::event_authority,
    shared::error::DequeError, state::EphemeralEventLog,
};

/// This doesn't actually need to do anything- it merely flushes the passed instruction data.
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Initialize event authority!");
    let ctx = EventAuthorityContext::load(accounts)?;

    // Skip initialized if it's already been done.
    if !ctx.event_authority.info.data_is_empty() {
        return Ok(());
    }

    let lamports_required = Rent::get()?.minimum_balance(MAX_PERMITTED_DATA_INCREASE);

    // Create the event authority PDA.
    invoke_signed(
        &system_instruction::create_account(
            ctx.payer.key,
            ctx.event_authority.info.key,
            lamports_required,
            MAX_PERMITTED_DATA_INCREASE as u64,
            program_id,
        ),
        &[
            ctx.payer.clone(),
            ctx.event_authority.info.clone(),
            ctx.system_program.info.clone(),
        ],
        &[event_authority::SEEDS],
    )?;

    let mut data = ctx
        .event_authority
        .info
        .try_borrow_mut_data()
        .or(Err(DequeError::InvalidEventAuthorityBorrow))?;
    // TODO: Consider if this would be much simpler/more ergonomic if the lifetime of the
    // ephemeral event log were tied to the EventAuthorityInfo struct.
    // It'd be easy to call, would reduce boilerplate, and the lifetimes would automatically
    // protect against double borrows (I believe).

    EphemeralEventLog::init(&mut data)?;

    Ok(())
}
