use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::state::DequeAccount;

pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo], index: u64) -> ProgramResult {
    msg!("Remove at index: {}", index);

    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;

    let mut data = deque_account.data.borrow_mut();
    let mut deque = DequeAccount::try_from_slice(&data)?;

    match &mut deque {
        DequeAccount::FiveU64s(d) => {
            let removed = d.remove(index).map_err(|_| ProgramError::Custom(2))?;
            msg!("Removed u64 value: {}", removed);
        }
        DequeAccount::TenU32s(d) => {
            let removed = d.remove(index).map_err(|_| ProgramError::Custom(2))?;
            msg!("Removed u32 value: {}", removed);
        }
    }

    deque.serialize(&mut &mut data[..])?;
    Ok(())
}
