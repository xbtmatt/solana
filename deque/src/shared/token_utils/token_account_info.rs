use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::require;

/// Represents an [associated token account](https://solana.com/docs/tokens#associated-token-account).
///
/// Both `spl_token` and `spl_token_22` have the same layout.
///
/// See: [`spl_token::state::Account`](https://docs.rs/spl-token/latest/spl_token/state/struct.Account.html).
///
/// See: [`spl_token_2022::state::Account`](https://docs.rs/spl-token-2022/latest/spl_token/state/struct.Account.html).
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
        require!(
            info.owner == &spl_token::id() || info.owner == &spl_token_2022::id(),
            ProgramError::IllegalOwner,
            "Associated token account owner must be owned by a token program."
        )?;

        require!(
            &info.try_borrow_data()?[32..64] == owner.as_ref(),
            ProgramError::IllegalOwner,
            "Token account owner doesn't match expected owner"
        )?;

        require!(
            &info.try_borrow_data()?[0..32] == mint.as_ref(),
            ProgramError::InvalidInstructionData,
            "Token account mint doesn't match expected mint"
        )?;

        Ok(TokenAccountInfo { info })
    }

    pub fn new_checked_owners_with_key(
        info: &'a AccountInfo<'info>,
        mint: &Pubkey,
        owner: &Pubkey,
        expected_pubkey: &Pubkey,
    ) -> Result<TokenAccountInfo<'a, 'info>, ProgramError> {
        if info.key != expected_pubkey {
            return Err(ProgramError::InvalidInstructionData);
        }
        Self::new_checked_owners(info, mint, owner)
    }

    /// Get an account's balance from the token amount in its associated token account data.
    ///
    /// ```
    /// pub struct Account {
    ///     pub mint: Pubkey, // 32
    ///     pub owner: Pubkey, // 32
    ///     pub amount: u64, // 8
    ///     // ...
    /// }
    /// ```
    pub fn get_balance(&self) -> u64 {
        let mut buf = [0u8; 8];
        let bytes = &self.info.try_borrow_data().expect("Should be borrowable")[64..72];
        buf.copy_from_slice(bytes);
        u64::from_le_bytes(buf)
    }

    /// Get the mint for an associated token account.
    ///
    /// ```
    /// pub struct Account {
    ///     pub mint: Pubkey, // 32
    ///     // ...
    /// }
    /// ```
    pub fn get_mint(&self) -> Pubkey {
        let mut buf = [0u8; 32];
        let bytes = &self.info.try_borrow_data().expect("Should be borrowable")[0..32];
        buf.copy_from_slice(bytes);
        Pubkey::new_from_array(buf)
    }

    /// Get the account that owns the *balance* inside the associated token account.
    ///
    /// That is, get the nominal owner of the tokens, not the literal account owner of the
    /// associated token account, which is one of the two token programs.
    ///
    /// ```
    /// pub struct Account {
    ///     pub mint: Pubkey, // 32
    ///     pub owner: Pubkey, // 32
    ///     // ...
    /// }
    /// ```
    pub fn get_owner(&self) -> Pubkey {
        let mut buf = [0u8; 32];
        let bytes = &self.info.try_borrow_data().expect("Should be borrowable")[32..64];
        buf.copy_from_slice(bytes);
        Pubkey::new_from_array(buf)
    }
}
