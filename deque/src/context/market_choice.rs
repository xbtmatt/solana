use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
};

use crate::{
    state::{Deque, MarketEscrowChoice},
    token_utils::TokenAccountInfo,
    utils::check_owned_and_writable,
};

#[derive(Clone)]
pub struct MarketChoiceContext<'a, 'info> {
    pub deque_account: &'a AccountInfo<'info>,
    pub payer: &'a AccountInfo<'info>,
    pub payer_ata: &'a AccountInfo<'info>,
    pub token_program: &'a AccountInfo<'info>,
    pub vault_ata: &'a AccountInfo<'info>,
}

impl<'a, 'info> MarketChoiceContext<'a, 'info> {
    pub fn load(
        accounts: &'a [AccountInfo<'info>],
        choice: &MarketEscrowChoice,
    ) -> Result<MarketChoiceContext<'a, 'info>, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let deque_account = next_account_info(accounts_iter)?;
        let payer = next_account_info(accounts_iter)?;
        let payer_ata = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        let mint_in = next_account_info(accounts_iter)?;
        let vault_ata = next_account_info(accounts_iter)?;

        let mut data = deque_account.data.borrow_mut();
        let deque = Deque::new_from_bytes(&mut data)?;
        check_owned_and_writable(deque_account)?;

        let mint = match choice {
            MarketEscrowChoice::Base => deque.header.base_mint,
            MarketEscrowChoice::Quote => deque.header.quote_mint,
        };

        // Ensure the mint pubkey passed into account data matches the mint in header data.
        if mint_in.key.as_ref() != mint.as_ref() {
            return Err(ProgramError::IllegalOwner);
        }

        let (payer_ata, vault_ata) = (
            TokenAccountInfo::new_checked_owners(payer_ata, &mint, payer.key)?.info,
            TokenAccountInfo::new_checked_owners(vault_ata, &mint, deque_account.key)?.info,
        );

        Ok(MarketChoiceContext {
            deque_account,
            payer,
            payer_ata,
            token_program,
            vault_ata,
        })
    }
}
