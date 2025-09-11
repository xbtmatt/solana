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
    state::{Deque, DequeType, HEADER_FIXED_SIZE},
    token_utils::create_token_vault,
    utils::check_derivations_and_get_bumps,
};

pub fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    deque_ty: u8,
    num_sectors: u16,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
) -> ProgramResult {
    let deque_type = DequeType::try_from(deque_ty)?;
    msg!(
        "Initialize deque type: {:#?} with {:?} sector(s)",
        deque_type,
        num_sectors
    );

    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let vault = next_account_info(accounts_iter)?;
    let token_program_account = next_account_info(accounts_iter)?;
    if token_program_account.key.as_array() != spl_token::id().as_array() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let (deque_bump, vault_bump) =
        check_derivations_and_get_bumps(deque_account, vault, base_mint, quote_mint)?;

    let sector_size = deque_type.sector_size();
    let account_space = HEADER_FIXED_SIZE + sector_size * (num_sectors as usize);
    let lamports_required = Rent::get()?.minimum_balance(account_space);

    // Create the deque PDA.
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            deque_account.key,
            lamports_required,
            account_space as u64,
            program_id,
        ),
        &[payer.clone(), deque_account.clone(), system_program.clone()],
        deque_seeds_with_bump!(base_mint, quote_mint, deque_bump),
    )?;

    // Create the token vault.
    create_token_vault(
        payer,
        system_program,
        token_program_account,
        deque_account.key,
        base_mint,
        quote_mint,
        (vault, vault_bump),
    )?;

    {
        let mut data = deque_account.try_borrow_mut_data()?;
        Deque::init_deque_account(
            &mut data,
            deque_type,
            num_sectors,
            deque_bump,
            (vault.key, vault_bump),
            base_mint,
            quote_mint,
        )?;
    }

    msg!(
        "Deque initialized successfully (space = {:?} bytes).",
        account_space
    );
    Ok(())
}
