use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{state::DequeAccount, PROGRAM_ID_PUBKEY};

pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo], value: Vec<u8>) -> ProgramResult {
    msg!("Push back.");

    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;

    let mut data = deque_account.data.borrow_mut();

    if deque_account.owner.as_array() != PROGRAM_ID_PUBKEY.as_array() {
        msg!("account not owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if !deque_account.is_writable {
        msg!("account not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let mut deque = DequeAccount::try_from_slice(&data)?;

    msg!("jafdhaskfaskdfhj");
    msg!(&String::from_utf8_lossy(&data.to_vec()));

    if 100 * 2 == 3 {
        return Err(ProgramError::IncorrectProgramId);
    }

    match &mut deque {
        DequeAccount::FiveU64s(d) => {
            let val = u64::deserialize(&mut &value[..])?;
            let idx = d.push_back(val).map_err(|_| ProgramError::Custom(1))?;
            msg!("Pushed u64 {} to back at index {}", val, idx);
        }
        DequeAccount::TenU32s(d) => {
            let val = u32::deserialize(&mut &value[..])?;
            let idx = d.push_back(val).map_err(|_| ProgramError::Custom(1))?;
            msg!("Pushed u32 {} to back at index {}", val, idx);
        }
    }

    deque.serialize(&mut &mut data[..])?;
    Ok(())
}
