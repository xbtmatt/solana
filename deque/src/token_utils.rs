use solana_program::{
    account_info::AccountInfo, program::invoke_signed, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey, rent::Rent, system_instruction::create_account,
    sysvar::Sysvar,
};

use crate::vault_seeds_with_bump;

pub fn create_token_vault<'a, 'info>(
    payer: &'a AccountInfo<'info>,
    system_program: &'a AccountInfo<'info>,
    token_program: &'a AccountInfo<'info>,
    deque: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    vault_ctx: (&'a AccountInfo<'info>, u8),
) -> Result<(), ProgramError> {
    let (vault, vault_bump) = vault_ctx;
    let space = spl_token::state::Account::LEN;

    invoke_signed(
        &create_account(
            payer.key,
            vault.key,
            Rent::get()?.minimum_balance(space),
            space as u64,
            token_program.key,
        ),
        &[payer.clone(), vault.clone(), system_program.clone()],
        vault_seeds_with_bump!(deque.as_ref().to_vec(), base_mint, quote_mint, vault_bump),
    )?;

    Ok(())
}
