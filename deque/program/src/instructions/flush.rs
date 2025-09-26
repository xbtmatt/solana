use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
};

use crate::{require, seeds};

/// This doesn't actually need to do anything- it merely flushes the passed instruction data.
pub fn process(accounts: &[AccountInfo]) -> ProgramResult {
    let authority = next_account_info(&mut accounts.iter())?;
    require!(
        authority.is_signer,
        ProgramError::MissingRequiredSignature,
        "Event authority must be a signer"
    )?;
    require!(
        authority.key.as_ref() == seeds::event_authority::ID.as_ref(),
        ProgramError::IncorrectAuthority,
        "Invalid event authority"
    )?;
    msg!("Flushing! 🚽");

    Ok(())
}
