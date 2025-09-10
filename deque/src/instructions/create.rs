use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::state::{Deque, DequeType, HEADER_FIXED_SIZE};

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    deque_ty: u8,
    num_slots: u16,
) -> ProgramResult {
    let deque_type = DequeType::try_from(deque_ty)?;
    msg!(
        "Initialize deque type: {:#?} with {:?} slot(s)",
        deque_type,
        num_slots
    );

    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let slot_size = deque_type.slot_size();
    let account_space = HEADER_FIXED_SIZE + slot_size * (num_slots as usize);
    let lamports_required = Rent::get()?.minimum_balance(account_space);

    invoke(
        &system_instruction::create_account(
            payer.key,
            deque_account.key,
            lamports_required,
            account_space as u64,
            program_id,
        ),
        &[payer.clone(), deque_account.clone(), system_program.clone()],
    )?;

    {
        let mut data = deque_account.try_borrow_mut_data()?;
        Deque::init_deque_account(&mut data, deque_type, num_slots)?;
    }

    msg!(
        "Deque initialized successfully (space = {:?} bytes).",
        account_space
    );
    Ok(())
}
