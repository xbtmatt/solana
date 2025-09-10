use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    state::{Deque, DequeType},
    utils::{check_owned_and_writable, SectorIndex},
};

pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    index: SectorIndex,
) -> ProgramResult {
    msg!("Remove at index: {}", index);

    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;

    let mut data = deque_account.data.borrow_mut();
    let mut deque = Deque::new_from_bytes(&mut data)?;

    check_owned_and_writable(deque_account)?;

    match deque.header.get_type() {
        DequeType::U32 => {
            let removed = deque
                .remove::<u32>(index)
                .map_err(|_| ProgramError::InvalidArgument)?;
            msg!("Removed U32 value: {}", removed);
        }
        DequeType::U64 => {
            let removed = deque
                .remove::<u64>(index)
                .map_err(|_| ProgramError::InvalidArgument)?;
            msg!("Removed U64 value: {}", removed);
        }
        DequeType::Market => {
            todo!();
        }
    }
    Ok(())
}
