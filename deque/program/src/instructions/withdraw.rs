use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    context::market_choice::MarketChoiceContext,
    instruction_enum::MarketEscrowChoice,
    shared::token_utils::vault_transfers::withdraw_from_vault,
    state::{Deque, MarketEscrow},
};

pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    choice: MarketEscrowChoice,
) -> ProgramResult {
    let ctx = MarketChoiceContext::load(accounts, choice)?;

    let MarketChoiceContext {
        deque_account,
        payer,
        ..
    } = ctx;

    let mut data = deque_account.data.borrow_mut();
    // Deque discriminant is checked in `load`.
    let deque = Deque::new_from_bytes_unchecked(&mut data)?;

    // Try to find a node with the payer.
    let escrow_and_idx = deque
        .iter_nodes::<MarketEscrow>()
        .find(|(node, _)| node.trader.as_ref() == payer.key.as_ref())
        .map(|(node, idx)| (*node, idx));

    // Drop the deque account data ref so it's possible to call transfer.
    drop(data);

    match escrow_and_idx {
        Some((escrow, idx)) => {
            let amount = escrow.amount_from_choice(&ctx.choice);
            withdraw_from_vault(&ctx, amount)?;

            let mut data = deque_account.data.borrow_mut();
            let mut deque = Deque::new_from_bytes_unchecked(&mut data)?;

            // And remove the node from the deque.
            deque
                .remove::<MarketEscrow>(idx)
                .map_err(|_| ProgramError::InvalidAccountData)?;
            msg!("Withdrawing {} coins", amount);
        }
        None => {
            msg!("Trader has no active escrow");
            return Err(ProgramError::InvalidArgument);
        }
    }

    Ok(())
}
