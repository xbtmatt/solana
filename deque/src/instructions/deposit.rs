use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    state::{Deque, DequeType, MarketEscrow, MarketEscrowChoice},
    token_utils::TokenAccountInfo,
    utils::check_owned_and_writable,
};

pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    choice: MarketEscrowChoice,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let deque_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let vault = next_account_info(accounts_iter)?;
    let payer_ata = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let mint_in = next_account_info(accounts_iter)?;
    let vault_ata = next_account_info(accounts_iter)?;

    let mut data = deque_account.data.borrow_mut();
    let mut deque = Deque::new_from_bytes(&mut data)?;

    // Ensure it's a market deque.
    match deque.header.get_type() {
        DequeType::Market => (),
        // Not supported. Will remove later.
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    check_owned_and_writable(deque_account)?;

    let (mint, base_amt, quote_amt) = match choice {
        MarketEscrowChoice::Base => (deque.header.base_mint, amount, 0),
        MarketEscrowChoice::Quote => (deque.header.quote_mint, 0, amount),
    };

    // Ensure the mint pubkey passed into account data matches the mint in header data.
    if mint_in.key.as_ref() != mint.as_ref() || vault.key.as_ref() != deque.header.vault.as_ref() {
        return Err(ProgramError::IllegalOwner);
    }

    let TokenAccountInfo {
        info: checked_payer_ata,
    } = TokenAccountInfo::new_checked_owners(payer_ata, &mint, payer.key)?;

    let TokenAccountInfo {
        info: checked_vault_ata,
    } = TokenAccountInfo::new_checked_owners(vault_ata, &mint, &deque.header.vault)?;

    // Transfer from the payer's token account to the vault's token account.
    invoke(
        &spl_token::instruction::transfer(
            token_program.key,
            checked_payer_ata.key,
            checked_vault_ata.key,
            payer.key,
            &[],
            amount,
        )?,
        &[
            token_program.as_ref().clone(),
            checked_payer_ata.as_ref().clone(),
            checked_vault_ata.as_ref().clone(),
            payer.as_ref().clone(),
        ],
    )?;

    // TODO: Check if the user already has existing funds within the deque.
    // Now push a node indicating that this user has escrowed tokens.
    deque
        .push_front(MarketEscrow::new(*payer.key, base_amt, quote_amt))
        .map_err(|_| ProgramError::InvalidAccountData)?;
    msg!("Pushed market escrow to front.");

    Ok(())
}
