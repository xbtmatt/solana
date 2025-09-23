use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    context::market_choice::MarketChoiceContext,
    events::{event_emitter::EventEmitter, WithdrawEventData},
    instruction_enum::MarketChoice,
    shared::token_utils::vault_transfers::withdraw_from_vault,
    state::{Deque, DequeNode, MarketEscrow},
    utils::from_sector_idx_mut,
};

pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    choice: MarketChoice,
    event_emitter: &mut EventEmitter,
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

    let amount = match escrow_and_idx {
        Some((escrow, idx)) => {
            let amount = escrow.amount_from_choice(&ctx.choice);

            if amount > 0 {
                withdraw_from_vault(&ctx, amount)?;
            }

            let mut data = deque_account.data.borrow_mut();
            let mut deque = Deque::new_from_bytes_unchecked(&mut data)?;

            // Remove the node from the deque if the trader has no coins in either token.
            if escrow.amount_of_opposite_choice(&ctx.choice) == 0 {
                msg!("Both amounts are 0. Removing node from the deque!");
                deque
                    .remove::<MarketEscrow>(idx)
                    .map_err(|_| ProgramError::InvalidAccountData)?;
            } else {
                // Otherwise, just zero out the one that was just withdrawn.
                msg!("Zeroing out the token that was withdrawn.");
                let node = from_sector_idx_mut::<DequeNode<MarketEscrow>>(deque.sectors, idx)?;
                match choice {
                    MarketChoice::Base => node.inner.base = 0,
                    MarketChoice::Quote => node.inner.quote = 0,
                };
            }

            msg!("Withdrawing {} coins", amount);

            amount
        }
        None => {
            msg!("Trader has no active escrow");
            return Err(ProgramError::InvalidArgument);
        }
    };

    event_emitter.add_event(WithdrawEventData::new(ctx.payer.key, amount, ctx.choice))?;

    Ok(())
}
