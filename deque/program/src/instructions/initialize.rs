use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    pubkey::Pubkey, rent::Rent, system_instruction, sysvar::Sysvar,
};

use crate::{
    context::initialize_deque::InitializeDequeContext,
    market_seeds_with_bump,
    shared::token_utils::create_vault::create_token_vault,
    state::{Deque, HEADER_FIXED_SIZE},
    utils::SECTOR_SIZE,
};

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], num_sectors: u16) -> ProgramResult {
    msg!("Initialize deque with {:?} sector(s)", num_sectors);

    let ctx = InitializeDequeContext::load(accounts)?;

    let account_space = HEADER_FIXED_SIZE + SECTOR_SIZE * (num_sectors as usize);
    let lamports_required = Rent::get()?.minimum_balance(account_space);

    // Create the deque PDA.
    invoke_signed(
        &system_instruction::create_account(
            ctx.payer.key,
            ctx.deque_account.key,
            lamports_required,
            account_space as u64,
            program_id,
        ),
        &[
            ctx.payer.clone(),
            ctx.deque_account.clone(),
            ctx.system_program.info.clone(),
        ],
        market_seeds_with_bump!(
            ctx.base_mint.info.key,
            ctx.quote_mint.info.key,
            ctx.market_bump
        ),
    )?;

    // Create the token vault.
    create_token_vault(
        ctx.payer,
        ctx.deque_account,
        (&ctx.base_mint, &ctx.quote_mint),
        (&ctx.vault_base_ata, &ctx.vault_quote_ata),
        (&ctx.base_token_program, &ctx.quote_token_program),
        &ctx.associated_token_program,
        &ctx.system_program,
    )?;

    {
        let mut data = ctx.deque_account.try_borrow_mut_data()?;
        Deque::init_deque_account(
            &mut data,
            num_sectors,
            ctx.market_bump,
            ctx.base_mint.info.key,
            ctx.quote_mint.info.key,
        )?;
    }

    msg!(
        "Deque initialized successfully (space = {:?} bytes).",
        account_space
    );
    Ok(())
}
