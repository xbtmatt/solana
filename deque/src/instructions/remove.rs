use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    state::{DequeAccount, Link},
    PROGRAM_ID_PUBKEY,
};

pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo], index: Link) -> ProgramResult {
    msg!("Remove at index: {}", index);

    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;

    let mut data = deque_account.data.borrow_mut();
    let mut deque = DequeAccount::try_from_slice(&data)?;

    if deque_account.owner.as_array() != PROGRAM_ID_PUBKEY.as_array() {
        msg!("account not owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if !deque_account.is_writable {
        msg!("account not writable");
        return Err(ProgramError::InvalidAccountData);
    }

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
