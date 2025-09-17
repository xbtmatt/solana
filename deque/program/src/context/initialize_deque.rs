use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
};

use crate::{
    state::{Deque, MarketEscrowChoice},
    utils::check_owned_and_writable,
    validation::{
        system_program::SystemProgramInfo,
        token_accounts::{TokenAccountInfo, TokenMintInfo, TokenProgramInfo},
    },
};

#[derive(Clone)]
pub struct InitializeDequeContext<'a, 'info> {
    pub deque_account: &'a AccountInfo<'info>,
    pub payer: &'a AccountInfo<'info>,
    pub base_mint: TokenMintInfo<'a, 'info>,
    pub quote_mint: TokenMintInfo<'a, 'info>,
    pub vault_base_ata: TokenAccountInfo<'a, 'info>,
    pub vault_quote_ata: TokenAccountInfo<'a, 'info>,
    pub base_token_program: TokenProgramInfo<'a, 'info>,
    pub quote_token_program: TokenProgramInfo<'a, 'info>,
    pub system_program: SystemProgramInfo<'a, 'info>,
}

impl<'a, 'info> InitializeDequeContext<'a, 'info> {
    pub fn load(
        accounts: &'a [AccountInfo<'info>],
        choice: MarketEscrowChoice,
    ) -> Result<InitializeDequeContext<'a, 'info>, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let deque_account = next_account_info(accounts_iter)?;
        let payer = next_account_info(accounts_iter)?;
        let base_mint = TokenMintInfo::new_checked(next_account_info(accounts_iter)?)?;
        let quote_mint = TokenMintInfo::new_checked(next_account_info(accounts_iter)?)?;
        let vault_base_ata = next_account_info(accounts_iter)?;
        let vault_quote_ata =
            TokenAccountInfo::new_checked_owners(next_account_info(accounts_iter)?, quote_mint);
        let base_token_program = TokenProgramInfo::new_checked(next_account_info(accounts_iter)?)?;
        let quote_token_program = TokenProgramInfo::new_checked(next_account_info(accounts_iter)?)?;
        let system_program = SystemProgramInfo::new_checked(next_account_info(accounts_iter)?)?;

        let mut data = deque_account.data.borrow_mut();
        let deque = Deque::new_from_bytes(&mut data)?;
        check_owned_and_writable(deque_account)?;

        let mint = match choice {
            MarketEscrowChoice::Base => deque.header.base_mint,
            MarketEscrowChoice::Quote => deque.header.quote_mint,
        };

        // Ensure the mint pubkey passed into account data matches the mint in header data.
        if mint_in.key.as_ref() != mint.as_ref() {
            return Err(ProgramError::InvalidInstructionData);
        }

        let (payer_ata, vault_ata, token_program, mint_info) = (
            TokenAccountInfo::new_checked_owners(payer_ata, &mint, payer.key)?,
            TokenAccountInfo::new_checked_owners(vault_ata, &mint, deque_account.key)?,
            TokenProgramInfo::new_checked(token_program)?,
            TokenMintInfo::new_checked(mint_in)?,
        );

        Ok(InitializeDequeContext {
            deque_account,
            payer,
            payer_ata,
            token_program,
            vault_ata,
            system_program,
            mint_info,
            choice,
        })
    }
}
