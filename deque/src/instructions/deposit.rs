use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke,
    program_error::ProgramError, pubkey::Pubkey,
};

use crate::{
    context::market_choice::MarketChoiceContext,
    state::{Deque, MarketEscrow, MarketEscrowChoice},
};

pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    choice: MarketEscrowChoice,
) -> ProgramResult {
    let MarketChoiceContext {
        deque_account,
        payer,
        payer_ata,
        token_program,
        vault_ata,
    } = MarketChoiceContext::load(accounts, &choice)?;

    let (base_amt, quote_amt) = match choice {
        MarketEscrowChoice::Base => (amount, 0),
        MarketEscrowChoice::Quote => (0, amount),
    };

    // Transfer from the payer's token account to the vault's token account.
    invoke(
        &spl_token::instruction::transfer(
            token_program.key,
            payer_ata.key,
            vault_ata.key,
            payer.key,
            &[],
            amount,
        )?,
        &[
            token_program.as_ref().clone(),
            payer_ata.as_ref().clone(),
            vault_ata.as_ref().clone(),
            payer.as_ref().clone(),
        ],
    )?;

    // TODO: Check if the user already has existing funds within the deque.
    // Ideally they're consolidated into a single node.
    // I believe it's possible to validate/verify that a client-passed-in memory address is a valid
    // node for a user without having to traverse it in the smart contract, since the sector sizes
    // will be fixed and aligned. Simply verify the pubkey matches and then operate from there.
    // If this is too complex and/or unsafe, just traverse the deque.

    // Now push a node indicating that this user has escrowed tokens.
    let mut data = deque_account.data.borrow_mut();
    let mut deque = Deque::new_from_bytes(&mut data)?;
    let escrow = MarketEscrow::new(*payer.key, base_amt, quote_amt);

    deque
        .push_front(escrow)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    Ok(())
}
