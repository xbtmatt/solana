use anyhow::Context;
use deque::{
    state::{get_deque_address, DequeInstruction, MarketEscrowChoice},
    PROGRAM_ID_PUBKEY,
};
use solana_client::rpc_client::RpcClient;
use solana_program::example_mocks::solana_sdk::system_program;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Mint;

use crate::transactions::send_txn;

/// Returns the mint pubkey and the token account pubkey.
pub fn create_token(
    rpc: &RpcClient,
    payer: &Keypair,
    mint_decimals: u8,
    mint_amt: u64,
) -> anyhow::Result<(Pubkey, Pubkey)> {
    let mint = Keypair::new();
    let mint_rent = rpc.get_minimum_balance_for_rent_exemption(Mint::LEN)?;
    let create_mint = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        Mint::LEN as u64,
        &spl_token::id(),
    );
    let init_mint = spl_token::instruction::initialize_mint2(
        &spl_token::id(),
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        mint_decimals,
    )
    .context("failed initialize_mint2")?;

    let payer_ata = get_associated_token_address(&payer.pubkey(), &mint.pubkey());
    let create_ata =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &payer.pubkey(),
            &payer.pubkey(),
            &mint.pubkey(),
            &spl_token::id(),
        );

    let mint_to = spl_token::instruction::mint_to_checked(
        &spl_token::id(),
        &mint.pubkey(),
        &payer_ata,
        &payer.pubkey(),
        &[],
        mint_amt,
        mint_decimals,
    )
    .context("failed mint_to_checked")?;

    send_txn(
        rpc,
        payer,
        &[&mint],
        vec![create_mint, init_mint],
        "--- create and initialize mint ---".to_string(),
    );
    send_txn(
        rpc,
        payer,
        &[payer],
        vec![create_ata, mint_to],
        "--- create ATA and mint to it".to_string(),
    );

    Ok((mint.pubkey(), payer_ata))
}

pub fn get_token_balance(rpc: &RpcClient, owner: &Pubkey, mint: &Pubkey) -> u64 {
    let ata = get_associated_token_address(owner, mint);
    let acc_data = rpc
        .get_account(&ata)
        .expect("Should be able to get account")
        .data;
    let token_account =
        spl_token::state::Account::unpack(&acc_data).expect("Should have account data.");

    token_account.amount
}

pub struct DequeContext {
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub deque_pubkey: Pubkey,
    pub vault_base_ata: Pubkey,
    pub vault_quote_ata: Pubkey,
    pub token_program: Pubkey,
    pub ata_program: Pubkey,
}

impl DequeContext {
    pub fn create_ata_ixn(&self, payer: &Keypair, choice: MarketEscrowChoice) -> Instruction {
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &payer.pubkey(),
            &payer.pubkey(),
            match choice {
                MarketEscrowChoice::Base => &self.base_mint,
                MarketEscrowChoice::Quote => &self.quote_mint,
            },
            &self.token_program,
        )
    }

    pub fn initialize_deque_ixn(&self, payer: &Keypair, num_sectors: u16) -> Instruction {
        Instruction::new_with_borsh(
            PROGRAM_ID_PUBKEY,
            &DequeInstruction::Initialize { num_sectors },
            vec![
                AccountMeta::new(self.deque_pubkey, false),
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(self.token_program, false),
                // For the ATA creation inside the program.
                AccountMeta::new(self.vault_base_ata, false),
                AccountMeta::new(self.vault_quote_ata, false),
                AccountMeta::new_readonly(self.base_mint, false),
                AccountMeta::new_readonly(self.quote_mint, false),
                AccountMeta::new_readonly(self.ata_program, false),
            ],
        )
    }
}

pub const INITIAL_MINT_AMOUNT: u64 = 100000;

pub fn generate_deque(rpc: &RpcClient, payer: &Keypair) -> anyhow::Result<DequeContext> {
    let (base_mint, _) =
        create_token(rpc, payer, 10, INITIAL_MINT_AMOUNT).context("failed to mint base")?;
    let (quote_mint, _) =
        create_token(rpc, payer, 10, INITIAL_MINT_AMOUNT).context("failed to mint quote")?;
    let (deque_pubkey, _) = get_deque_address(&base_mint, &quote_mint);

    // ------------------------------------- Initialization ----------------------------------------
    let (vault_base_ata, vault_quote_ata) = (
        get_associated_token_address(&deque_pubkey, &base_mint),
        get_associated_token_address(&deque_pubkey, &quote_mint),
    );

    Ok(DequeContext {
        base_mint,
        quote_mint,
        deque_pubkey,
        vault_base_ata,
        vault_quote_ata,
        token_program: spl_token::id(),
        ata_program: spl_associated_token_account::id(),
    })
}
