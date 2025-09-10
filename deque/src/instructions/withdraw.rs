use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

pub fn process(_program_id: &Pubkey, accounts: &[AccountInfo], _amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let _ = next_account_info(accounts_iter);
    todo!();
}
