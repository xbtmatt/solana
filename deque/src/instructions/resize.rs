use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    state::{Deque, DequeType, Stack, HEADER_FIXED_SIZE},
    utils::{check_owned_and_writable, SectorIndex},
};

pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo], num_sectors: u16) -> ProgramResult {
    if num_sectors < 1 {
        return Err(ProgramError::InvalidArgument);
    }
    msg!("Adding {} sectors.", num_sectors);

    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    check_owned_and_writable(deque_account)?;

    let mut deque_data = deque_account.try_borrow_mut_data()?;
    let current_size = deque_data.len();
    let current_lamports = deque_account.lamports();

    let deque = Deque::new_from_bytes(&mut deque_data)?;
    let deque_type = deque.header.get_type();
    let sector_size = deque_type.sector_size();

    drop(deque_data);

    let additional_space = sector_size * (num_sectors as usize);
    let new_account_space = current_size + additional_space;

    let new_lamports_required = Rent::get()?.minimum_balance(new_account_space);

    let lamports_diff = new_lamports_required.saturating_sub(current_lamports);

    if lamports_diff > 0 {
        invoke(
            &system_instruction::transfer(payer_account.key, deque_account.key, lamports_diff),
            &[
                payer_account.clone(),
                deque_account.clone(),
                system_program.clone(),
            ],
        )?;
    }

    // Zero init is a waste of compute unless an account shrinks and then grows in the same txn.
    // See: https://solana.com/developers/courses/program-optimization/program-architecture#data-optimization
    //
    // > Memory used to grow is already zero-initialized upon program entrypoint and re-zeroing it wastes compute units.
    // > If within the same call a program reallocs from larger to smaller and back to larger again the new space could contain stale data.
    // > Pass true for zero_init in this case, otherwise compute units will be wasted re-zero-initializing.
    deque_account.realloc(new_account_space, false)?;

    // Now chain the old sectors to the new sectors in the stack of free nodes.
    let mut deque_data = deque_account.data.borrow_mut();
    let deque = Deque::new_from_bytes(&mut deque_data)?;

    let curr_n_sectors = (current_size - HEADER_FIXED_SIZE) / sector_size;
    let new_n_sectors = curr_n_sectors + num_sectors as usize;

    // NOTE: This currently is O(n) writes for n new sectors. Technically, this could be O(1) by
    // overlaying the slab directly
    match deque_type {
        DequeType::U32 => {
            let mut free = Stack::<u32>::new(deque.sectors, deque.header.free_head);
            for i in curr_n_sectors..new_n_sectors {
                free.push_to_free(i as SectorIndex)?;
            }
            deque.header.free_head = free.get_head();
        }
        DequeType::U64 => {
            let mut free = Stack::<u64>::new(deque.sectors, deque.header.free_head);
            for i in curr_n_sectors..new_n_sectors {
                free.push_to_free(i as SectorIndex)?;
            }
            deque.header.free_head = free.get_head();
        }
        DequeType::Market => {
            todo!();
        }
    }

    Ok(())
}
