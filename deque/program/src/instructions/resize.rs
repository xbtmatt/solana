use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
};

use crate::utils::inline_resize;

pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo], num_sectors: u16) -> ProgramResult {
    msg!("Trying to add {} sectors.", num_sectors);

    let accounts_iter = &mut accounts.iter();
    let payer_account = next_account_info(accounts_iter)?;
    let deque_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    inline_resize(deque_account, payer_account, system_program, num_sectors)?;

    Ok(())
}
