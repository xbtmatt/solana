use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
};

use crate::{
    utils::check_derivations_and_get_bump,
    validation::{
        system_program::SystemProgramInfo,
        token_accounts::{AssociatedTokenProgramInfo, TokenMintInfo, TokenProgramInfo},
        uninitialized_account::UninitializedAccountInfo,
    },
};

#[derive(Clone)]
pub struct InitializeDequeContext<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub deque_account: &'a AccountInfo<'info>,
    pub base_mint: TokenMintInfo<'a, 'info>,
    pub quote_mint: TokenMintInfo<'a, 'info>,
    pub vault_base_ata: UninitializedAccountInfo<'a, 'info>,
    pub vault_quote_ata: UninitializedAccountInfo<'a, 'info>,
    pub base_token_program: TokenProgramInfo<'a, 'info>,
    pub quote_token_program: TokenProgramInfo<'a, 'info>,
    pub associated_token_program: AssociatedTokenProgramInfo<'a, 'info>,
    pub system_program: SystemProgramInfo<'a, 'info>,
    pub deque_bump: u8,
}

impl<'a, 'info> InitializeDequeContext<'a, 'info> {
    pub fn load(
        accounts: &'a [AccountInfo<'info>],
    ) -> Result<InitializeDequeContext<'a, 'info>, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let deque_account = next_account_info(accounts_iter)?;
        let base_mint = TokenMintInfo::new_checked(next_account_info(accounts_iter)?)?;
        let quote_mint = TokenMintInfo::new_checked(next_account_info(accounts_iter)?)?;
        let vault_base_ata =
            UninitializedAccountInfo::new_checked(next_account_info(accounts_iter)?)?;
        let vault_quote_ata =
            UninitializedAccountInfo::new_checked(next_account_info(accounts_iter)?)?;
        let base_token_program = TokenProgramInfo::new_checked(next_account_info(accounts_iter)?)?;
        let quote_token_program = TokenProgramInfo::new_checked(next_account_info(accounts_iter)?)?;
        let associated_token_program =
            AssociatedTokenProgramInfo::new_checked(next_account_info(accounts_iter)?)?;
        let system_program = SystemProgramInfo::new_checked(next_account_info(accounts_iter)?)?;

        let deque_bump =
            check_derivations_and_get_bump(deque_account, base_mint.info.key, quote_mint.info.key)?;

        Ok(InitializeDequeContext {
            payer,
            deque_account,
            base_mint,
            quote_mint,
            vault_base_ata,
            vault_quote_ata,
            base_token_program,
            quote_token_program,
            associated_token_program,
            system_program,
            deque_bump,
        })
    }
}
