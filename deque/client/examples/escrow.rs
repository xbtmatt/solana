use anyhow::Context;
use deque::instruction_enum::{
    DepositInstructionData, DequeInstruction, MarketChoice, WithdrawInstructionData,
};
use deque_client::{
    events::fetch_parsed_txn,
    fuzz::fuzz,
    initialize::initialize_deque_with_ctx,
    tokens::generate_market,
    transactions::{fund_account, send_deposit_or_withdraw, send_txn},
};
use itertools::Itertools;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to local cluster
    let rpc_url = String::from("http://localhost:8899");
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let payer = fund_account(&rpc, None).await.expect("Should fund account");
    test_market_escrow(&rpc, &payer).context("Market escrow test failed")
}

fn test_market_escrow(rpc: &RpcClient, payer: &Keypair) -> anyhow::Result<()> {
    // ----------------------- Mint two tokens and generate deque address --------------------------
    let ctx = generate_market(rpc, payer).expect("Should be able to generate deque");
    let payer_base_ata = get_associated_token_address(&payer.pubkey(), &ctx.base_mint);
    let _payer_quote_ata = get_associated_token_address(&payer.pubkey(), &ctx.quote_mint);

    println!("deque pubkey {:#?}", ctx.deque_pubkey.to_string());
    println!("base mint pubkey {:#?}", ctx.base_mint.to_string());
    println!("quote mint pubkey {:#?}", ctx.quote_mint.to_string());
    println!("payer_base_ata {:#?}", payer_base_ata.to_string());

    // ------------------------------------- Initialization ----------------------------------------
    initialize_deque_with_ctx(rpc, payer, &ctx)?;

    // ------------------------------ Deposit base, base, quote --------------------------------

    let ixns = vec![
        DepositInstructionData::new(1000, MarketChoice::Base).into(),
        DepositInstructionData::new(23458, MarketChoice::Quote).into(),
        DepositInstructionData::new(184, MarketChoice::Base).into(),
        WithdrawInstructionData::new(MarketChoice::Base).into(),
        WithdrawInstructionData::new(MarketChoice::Quote).into(),
    ]
    .into_iter()
    .map(|ixn_data| ctx.deposit_or_withdraw_ixn(payer, ixn_data))
    .collect_vec();

    let parsed_txn = send_txn(
        rpc,
        payer,
        &[payer],
        ixns,
        "lots of deposits/withdraws".to_string(),
    )
    .and_then(|sig| fetch_parsed_txn(rpc, sig))?;

    let deque_events = parsed_txn.get_inner_deque_events()?;
    println!("{:#?}", deque_events);

    // ----------------------------------------- Deposit -------------------------------------------
    let parsed_txn = send_deposit_or_withdraw(
        rpc,
        payer,
        ctx.deque_pubkey,
        payer_base_ata,
        ctx.base_mint,
        ctx.vault_base_ata,
        &DequeInstruction::Deposit(DepositInstructionData {
            amount: 1000,
            choice: MarketChoice::Base,
        }),
    )
    .and_then(|sig| fetch_parsed_txn(rpc, sig))?;

    let deque_events = parsed_txn.get_inner_deque_events()?;
    println!("{:#?}", deque_events);

    // ----------------------------------------- Withdraw -------------------------------------------
    let parsed_txn = send_deposit_or_withdraw(
        rpc,
        payer,
        ctx.deque_pubkey,
        payer_base_ata,
        ctx.base_mint,
        ctx.vault_base_ata,
        &DequeInstruction::Withdraw(WithdrawInstructionData {
            choice: MarketChoice::Base,
        }),
    )
    .map(|sig| fetch_parsed_txn(rpc, sig))??;

    let deque_events = parsed_txn.get_inner_deque_events()?;
    println!("{:#?}", deque_events);

    // // ----------------------------------------- Deposit -------------------------------------------
    let parsed_txn = send_deposit_or_withdraw(
        rpc,
        payer,
        ctx.deque_pubkey,
        payer_base_ata,
        ctx.base_mint,
        ctx.vault_base_ata,
        &DequeInstruction::Deposit(DepositInstructionData {
            amount: 1000,
            choice: MarketChoice::Base,
        }),
    )
    .map(|sig| fetch_parsed_txn(rpc, sig))??;

    let deque_events = parsed_txn.get_inner_deque_events()?;
    println!("{:#?}", deque_events);

    // ------------------------------------------- Fuzz --------------------------------------------
    const ROUNDS: u64 = 0;
    fuzz(rpc, payer, ctx, ROUNDS)?;

    Ok(())
}
