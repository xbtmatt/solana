use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, program::invoke};

pub fn create_token_vault<'a, 'info>(
    payer: &'a AccountInfo<'info>,
    deque_account: &'a AccountInfo<'info>,
    base_and_quote_mints: (&'a AccountInfo<'info>, &'a AccountInfo<'info>),
    vault_base_and_quote_atas: (&'a AccountInfo<'info>, &'a AccountInfo<'info>),
    program_accounts: (
        &'a AccountInfo<'info>,
        &'a AccountInfo<'info>,
        &'a AccountInfo<'info>,
    ),
) -> ProgramResult {
    let (token_program, spl_ata_program, system_program) = program_accounts;
    let (base_mint, quote_mint) = base_and_quote_mints;
    let (vault_base_ata, vault_quote_ata) = vault_base_and_quote_atas;

    // Then create the associated token accounts for the vault.
    for (ata, mint) in [(vault_base_ata, base_mint), (vault_quote_ata, quote_mint)] {
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account(
                payer.key,
                deque_account.key,
                mint.key,
                token_program.key,
            ),
            &[
                payer.clone(),
                ata.clone(),
                deque_account.clone(),
                mint.clone(),
                system_program.clone(),
                spl_ata_program.clone(),
                token_program.clone(),
            ],
        )?;
    }

    Ok(())
}
