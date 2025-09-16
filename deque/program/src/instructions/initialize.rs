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
    shared::token_utils::create_vault::create_token_vault,
    state::{Deque, HEADER_FIXED_SIZE},
    utils::{check_derivations_and_get_bump, SECTOR_SIZE},
};

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], num_sectors: u16) -> ProgramResult {
    msg!("Initialize deque with {:?} sector(s)", num_sectors);

    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let vault_base_ata = next_account_info(accounts_iter)?;
    let vault_quote_ata = next_account_info(accounts_iter)?;
    let base_mint_acc = next_account_info(accounts_iter)?;
    let quote_mint_acc = next_account_info(accounts_iter)?;
    let spl_ata_program = next_account_info(accounts_iter)?;

    if token_program.key.as_array() != spl_token::id().as_array() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let deque_bump =
        check_derivations_and_get_bump(deque_account, base_mint_acc.key, quote_mint_acc.key)?;

    let account_space = HEADER_FIXED_SIZE + SECTOR_SIZE * (num_sectors as usize);
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
        deque_seeds_with_bump!(base_mint_acc.key, quote_mint_acc.key, deque_bump),
    )?;

    // Create the token vault.
    create_token_vault(
        payer,
        deque_account,
        (base_mint_acc, quote_mint_acc),
        (vault_base_ata, vault_quote_ata),
        (token_program, spl_ata_program, system_program),
    )?;

    {
        let mut data = deque_account.try_borrow_mut_data()?;
        Deque::init_deque_account(
            &mut data,
            num_sectors,
            deque_bump,
            base_mint_acc.key,
            quote_mint_acc.key,
        )?;
    }

    msg!(
        "Deque initialized successfully (space = {:?} bytes).",
        account_space
    );
    Ok(())
}
