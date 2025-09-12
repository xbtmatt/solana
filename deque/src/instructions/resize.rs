use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    deque_seeds_with_bump,
    state::{Deque, MarketEscrow, Stack, HEADER_FIXED_SIZE},
    utils::{check_owned_and_writable, SectorIndex, SECTOR_SIZE},
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
    let current_size = deque_account.data_len();
    let current_lamports = deque_account.lamports();

    let deque = Deque::new_from_bytes(&mut deque_data)?;

    let (base_mint, quote_mint, deque_bump) = (
        deque.header.base_mint,
        deque.header.quote_mint,
        deque.header.deque_bump,
    );

    let new_account_space = current_size + SECTOR_SIZE * (num_sectors as usize);
    let new_lamports_required = Rent::get()?.minimum_balance(new_account_space);
    let lamports_diff = new_lamports_required.saturating_sub(current_lamports);

    drop(deque_data);

    if lamports_diff > 0 {
        invoke_signed(
            &system_instruction::transfer(payer_account.key, deque_account.key, lamports_diff),
            &[
                payer_account.clone(),
                deque_account.clone(),
                system_program.clone(),
            ],
            deque_seeds_with_bump!(base_mint, quote_mint, deque_bump),
        )?;
    }

    // "Memory used to grow is already zero-initialized upon program entrypoint and re-zeroing it wastes compute units."
    // See: https://solana.com/developers/courses/program-optimization/program-architecture#data-optimization
    deque_account.realloc(new_account_space, false)?;

    // Now chain the old sectors to the new sectors in the stack of free nodes.
    let mut deque_data = deque_account.data.borrow_mut();
    let deque = Deque::new_from_bytes_unchecked(&mut deque_data)?;

    let curr_n_sectors = (current_size - HEADER_FIXED_SIZE) / SECTOR_SIZE;
    let new_n_sectors = curr_n_sectors + num_sectors as usize;

    let mut free = Stack::<MarketEscrow>::new(deque.sectors, deque.header.free_head);
    for i in curr_n_sectors..new_n_sectors {
        free.push_to_free(i as SectorIndex)?;
    }
    deque.header.free_head = free.get_head();

    Ok(())
}
