use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, program::invoke};

use crate::validation::{
    system_program::SystemProgramInfo,
    token_accounts::{AssociatedTokenProgramInfo, TokenMintInfo, TokenProgramInfo},
    uninitialized_account::UninitializedAccountInfo,
};

pub fn create_token_vault<'a, 'info>(
    payer: &'a AccountInfo<'info>,
    deque_account: &'a AccountInfo<'info>,
    base_and_quote_mints: (&TokenMintInfo<'a, 'info>, &TokenMintInfo<'a, 'info>),
    vault_base_and_quote_atas: (
        &UninitializedAccountInfo<'a, 'info>,
        &UninitializedAccountInfo<'a, 'info>,
    ),
    base_and_quote_programs: (&TokenProgramInfo<'a, 'info>, &TokenProgramInfo<'a, 'info>),
    associated_token_program: &AssociatedTokenProgramInfo<'a, 'info>,
    system_program: &SystemProgramInfo<'a, 'info>,
) -> ProgramResult {
    let (base_program, quote_program) = base_and_quote_programs;
    let (base_mint, quote_mint) = base_and_quote_mints;
    let (vault_base_ata, vault_quote_ata) = vault_base_and_quote_atas;

    // Then create the associated token accounts for the vault.
    for (ata, mint, token_program) in [
        (vault_base_ata, base_mint, base_program),
        (vault_quote_ata, quote_mint, quote_program),
    ] {
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account(
                payer.key,
                deque_account.key,
                mint.info.key,
                token_program.info.key,
            ),
            &[
                payer.clone(),
                ata.info.clone(),
                deque_account.clone(),
                mint.info.clone(),
                system_program.info.clone(),
                associated_token_program.info.clone(),
                token_program.info.clone(),
            ],
        )?;
    }

    Ok(())
}
