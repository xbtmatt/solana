use borsh::{to_vec, BorshSerialize};
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
    log_bytes,
    state::{Deque, DequeAccount},
};

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], deque_type: u8) -> ProgramResult {
    msg!("Initialize deque type: {}", deque_type);

    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let deque = match deque_type {
        0 => DequeAccount::FiveU64s(Deque::<u64, 5>::new()),
        1 => DequeAccount::TenU32s(Deque::<u32, 10>::new()),
        _ => return Err(ProgramError::InvalidInstructionData),
    };

    let serialized = to_vec(&deque)?;
    msg!("logging serializeed bytes in create");
    log_bytes(&serialized);

    let account_span = serialized.len();
    let lamports_required = (Rent::get()?).minimum_balance(account_span);

    invoke(
        &system_instruction::create_account(
            payer.key,
            deque_account.key,
            lamports_required,
            account_span as u64,
            program_id,
        ),
        &[payer.clone(), deque_account.clone(), system_program.clone()],
    )?;

    deque.serialize(&mut &mut deque_account.data.borrow_mut()[..])?;

    msg!("Deque initialized successfully.");
    Ok(())
}
