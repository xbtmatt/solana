use solana_program::{
    account_info::AccountInfo,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction::create_account,
    sysvar::Sysvar,
};

use crate::vault_seeds_with_bump;

pub fn create_token_vault<'a, 'info>(
    payer: &'a AccountInfo<'info>,
    deque: &Pubkey,
    base_and_quote_mints: (&'a AccountInfo<'info>, &'a AccountInfo<'info>),
    vault_base_and_quote_atas: (&'a AccountInfo<'info>, &'a AccountInfo<'info>),
    vault_ctx: (&'a AccountInfo<'info>, u8),
    program_accounts: (
        &'a AccountInfo<'info>,
        &'a AccountInfo<'info>,
        &'a AccountInfo<'info>,
    ),
) -> Result<(), ProgramError> {
    let (token_program, spl_ata_program, system_program) = program_accounts;
    let (base_mint, quote_mint) = base_and_quote_mints;
    let (vault_base_ata, vault_quote_ata) = vault_base_and_quote_atas;
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
        vault_seeds_with_bump!(
            deque.as_ref().to_vec(),
            base_mint.key,
            quote_mint.key,
            vault_bump
        ),
    )?;

    // Then create the associated token accounts for the vault.
    for (ata, mint) in [(vault_base_ata, base_mint), (vault_quote_ata, quote_mint)] {
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account(
                payer.key,
                vault.key,
                mint.key,
                token_program.key,
            ),
            &[
                payer.clone(),
                ata.clone(),
                vault.clone(),
                mint.clone(),
                system_program.clone(),
                spl_ata_program.clone(),
                token_program.clone(),
            ],
        )?;
    }

    Ok(())
}

#[derive(Clone)]
pub struct TokenAccountInfo<'a, 'info> {
    pub info: &'a AccountInfo<'info>,
}

impl<'a, 'info> TokenAccountInfo<'a, 'info> {
    pub fn new_checked_owners(
        info: &'a AccountInfo<'info>,
        mint: &Pubkey,
        owner: &Pubkey,
    ) -> Result<TokenAccountInfo<'a, 'info>, ProgramError> {
        // TODO: Add spl_token_2022 support here.
        // The account owner should be a token program.
        if info.owner != &spl_token::id() ||
            // Mint pubkeys are at the 0 byte offset of the token account data. Verify it matches.
            &info.try_borrow_data()?[0..32] != mint.as_ref() ||
            // Token owner pubkeys are at the 32 byte offset of the token account data.
            &info.try_borrow_data()?[32..64] != owner.as_ref()
        {
            return Err(ProgramError::IllegalOwner);
        }
        let token_acc_info = TokenAccountInfo { info };

        Ok(token_acc_info)
    }
}
