use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    context::market_choice::MarketChoiceContext,
    events::{event_emitter::EventEmitter, DepositEventData},
    instruction_enum::MarketChoice,
    shared::token_utils::vault_transfers::deposit_to_vault,
    state::{Deque, DequeNode, MarketEscrow},
    utils::{from_sector_idx_mut, inline_deque_resize},
};

pub fn process(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount_in: u64,
    choice: MarketChoice,
    event_emitter: &mut EventEmitter,
) -> ProgramResult {
    let ctx = MarketChoiceContext::load(accounts, choice)?;

    let amount = deposit_to_vault(&ctx, amount_in)?;

    let MarketChoiceContext {
        deque_account,
        payer,
        system_program,
        choice,
        ..
    } = ctx;

    // Try to find the trader in existing nodes.
    let mut data = deque_account.data.borrow_mut();
    // The deque's account discriminant is checked in `load`.
    let deque = Deque::from_bytes_unchecked(&mut data)?;
    let maybe_idx = deque
        .iter_nodes::<MarketEscrow>()
        .find(|(node, _)| node.trader.as_ref() == payer.key.as_ref())
        .map(|(_, idx)| idx);
    let needs_resize = deque.header.len >= deque.get_capacity();

    match maybe_idx {
        // Mutate the node.
        Some(idx) => {
            let node = from_sector_idx_mut::<DequeNode<MarketEscrow>>(deque.sectors, idx)?;
            match choice {
                // Update the base amount in the existing node.
                MarketChoice::Base => {
                    node.inner.base = node
                        .inner
                        .base
                        .checked_add(amount)
                        .ok_or(ProgramError::InvalidArgument)?;
                }
                // Update the quote amount in the existing node.
                MarketChoice::Quote => {
                    node.inner.quote = node
                        .inner
                        .quote
                        .checked_add(amount)
                        .ok_or(ProgramError::InvalidArgument)?;
                }
            }
        }
        // Push a new node to the front of the deque.
        None => {
            drop(data);

            // Resize (grow) the account if there's not enough space.
            if needs_resize {
                msg!("Growing account by 1 sector");
                inline_deque_resize(deque_account, payer, system_program, 1)?;
            }

            let mut data = deque_account.data.borrow_mut();
            let mut deque = Deque::from_bytes_unchecked(&mut data)?;

            let escrow = match choice {
                MarketChoice::Base => MarketEscrow::new(*payer.key, amount, 0),
                MarketChoice::Quote => MarketEscrow::new(*payer.key, 0, amount),
            };

            deque
                .push_front(escrow)
                .map_err(|_| ProgramError::InvalidAccountData)?;
        }
    }

    event_emitter.add_event(DepositEventData::new(ctx.payer.key, amount, choice))?;

    Ok(())
}
