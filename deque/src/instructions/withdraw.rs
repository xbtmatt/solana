use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey,
};

use crate::{
    context::market_choice::MarketChoiceContext,
    state::{Deque, MarketEscrow, MarketEscrowChoice},
    utils::log_bytes,
    vault_seeds_with_bump,
};

pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    choice: MarketEscrowChoice,
) -> ProgramResult {
    let MarketChoiceContext {
        deque_account,
        payer,
        vault,
        payer_ata,
        token_program,
        vault_ata,
    } = MarketChoiceContext::load(accounts, &choice)?;

    let mut data = deque_account.data.borrow_mut();
    let mut deque = Deque::new_from_bytes(&mut data)?;

    // Try to find a node with the payer.
    let escrow_and_idx = deque
        .iter_nodes::<MarketEscrow>()
        .find(|(node, _)| node.trader.as_ref() == payer.key.as_ref())
        .map(|(node, idx)| (*node, idx));

    match escrow_and_idx {
        Some((escrow, idx)) => {
            let amount = escrow.amount_from_choice(choice);
            // Transfer from the vault to the payer's token account.
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program.key,
                    vault_ata.key,
                    payer_ata.key,
                    vault.key,
                    &[],
                    amount,
                )?,
                &[
                    token_program.as_ref().clone(),
                    vault_ata.as_ref().clone(),
                    payer_ata.as_ref().clone(),
                    vault.as_ref().clone(),
                ],
                vault_seeds_with_bump!(
                    &deque_account.key,
                    &deque.header.base_mint,
                    &deque.header.quote_mint,
                    deque.header.vault_bump
                ),
            )?;

            // And remove the node from the deque.
            deque
                .remove::<MarketEscrow>(idx)
                .map_err(|_| ProgramError::InvalidAccountData)?;
            msg!("Withdrawing {} coins");
        }
        None => {
            msg!("Trader has no active escrow");
        }
    }

    Ok(())
}
