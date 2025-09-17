use solana_program::{
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
};

use crate::{
    context::market_choice::MarketChoiceContext, deque_seeds_with_bump,
    loaders::token_accounts::TokenProgram, state::Deque,
};

pub fn deposit_to_vault<'a, 'info>(
    ctx: &'a MarketChoiceContext<'a, 'info>,
    amount: u64,
) -> Result<u64, ProgramError> {
    match ctx.token_program.program_type {
        TokenProgram::SplToken => {
            invoke(
                &spl_token::instruction::transfer(
                    ctx.token_program.info.key,
                    ctx.payer_ata.info.key,
                    ctx.vault_ata.info.key,
                    ctx.payer.key,
                    &[],
                    amount,
                )?,
                &[
                    ctx.token_program.info.as_ref().clone(),
                    ctx.payer_ata.info.as_ref().clone(),
                    ctx.vault_ata.info.as_ref().clone(),
                    ctx.payer.as_ref().clone(),
                ],
            )?;
            // `spl_token` always transfers the amount passed in.
            Ok(amount)
        }
        TokenProgram::SplToken2022 => {
            let mint_decimals = ctx.mint_info.get_decimals();
            let balance_before = ctx.vault_ata.get_balance();
            invoke(
                &spl_token_2022::instruction::transfer_checked(
                    ctx.token_program.info.key,
                    ctx.payer_ata.info.key,
                    ctx.mint_info.info.key,
                    ctx.vault_ata.info.key,
                    ctx.payer.key,
                    &[],
                    amount,
                    mint_decimals,
                )?,
                &[
                    ctx.token_program.info.as_ref().clone(),
                    ctx.payer_ata.info.as_ref().clone(),
                    ctx.vault_ata.info.as_ref().clone(),
                    ctx.payer.as_ref().clone(),
                ],
            )?;
            let balance_after = ctx.vault_ata.get_balance();
            // `spl_token_2022` amount deposited must be checked due to transfer hooks,
            // fees, and other misc extensions.
            let deposited = balance_after
                .checked_sub(balance_before)
                .ok_or(ProgramError::InvalidArgument)?;
            Ok(deposited)
        }
    }
}

pub fn withdraw_from_vault<'a, 'info>(
    ctx: &'a MarketChoiceContext<'a, 'info>,
    amount: u64,
) -> ProgramResult {
    let mut data = ctx.deque_account.data.borrow_mut();
    let deque = Deque::new_from_bytes_unchecked(&mut data)?;
    let (base_mint, quote_mint, deque_bump) = (
        deque.header.base_mint,
        deque.header.quote_mint,
        deque.header.deque_bump,
    );

    drop(data);

    match ctx.token_program.program_type {
        TokenProgram::SplToken => invoke_signed(
            &spl_token::instruction::transfer(
                ctx.token_program.info.key,
                ctx.vault_ata.info.key,
                ctx.payer_ata.info.key,
                ctx.deque_account.key,
                &[],
                amount,
            )?,
            &[
                ctx.token_program.info.as_ref().clone(),
                ctx.vault_ata.info.as_ref().clone(),
                ctx.payer_ata.info.as_ref().clone(),
                ctx.deque_account.as_ref().clone(),
            ],
            deque_seeds_with_bump!(base_mint, quote_mint, deque_bump),
        ),
        TokenProgram::SplToken2022 => {
            let mint_decimals = ctx.mint_info.get_decimals();
            invoke_signed(
                &spl_token_2022::instruction::transfer_checked(
                    ctx.token_program.info.key,
                    ctx.vault_ata.info.key,
                    ctx.mint_info.info.key,
                    ctx.payer_ata.info.key,
                    ctx.deque_account.key,
                    &[],
                    amount,
                    mint_decimals,
                )?,
                &[
                    ctx.token_program.info.as_ref().clone(),
                    ctx.vault_ata.info.as_ref().clone(),
                    ctx.payer_ata.info.as_ref().clone(),
                    ctx.deque_account.as_ref().clone(),
                ],
                deque_seeds_with_bump!(base_mint, quote_mint, deque_bump),
            )
        }
    }
}
