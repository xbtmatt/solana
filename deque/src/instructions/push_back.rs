use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    state::{Deque, DequeType},
    utils::check_owned_and_writable,
};

pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo], value: Vec<u8>) -> ProgramResult {
    msg!("Push back.");

    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;

    let mut data = deque_account.data.borrow_mut();
    let mut deque = Deque::new_from_bytes(&mut data)?;

    check_owned_and_writable(deque_account)?;

    match deque.header.get_type() {
        DequeType::U32 => {
            let val = u32::deserialize(&mut &value[..])?;
            let idx = deque
                .push_back(val)
                .map_err(|_| ProgramError::InvalidArgument)?;
            msg!("Pushed u64 {} to back at index {}", val, idx);
        }
        DequeType::U64 => {
            let val = u64::deserialize(&mut &value[..])?;
            let idx = deque
                .push_back(val)
                .map_err(|_| ProgramError::InvalidArgument)?;
            msg!("Pushed u64 {} to back at index {}", val, idx);
        }
        DequeType::Market => {
            todo!();
        }
    }
    Ok(())
}
