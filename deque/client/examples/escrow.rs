use anyhow::Context;
use deque::instruction_enum::{
    DepositInstructionData, DequeInstruction, MarketChoice, WithdrawInstructionData,
};
use deque_client::{
    events::fetch_parsed_txn,
    logs::print_size_and_sectors,
    tokens::{generate_market, DequeContext, INITIAL_MINT_AMOUNT},
    transactions::{fund_account, send_deposit_or_withdraw, send_txn},
};
use itertools::Itertools;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
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
    // Create both payer ATAs.
    send_txn(
        rpc,
        payer,
        &[payer],
        vec![
            ctx.create_ata_ixn(payer, MarketChoice::Base),
            ctx.create_ata_ixn(payer, MarketChoice::Quote),
            ctx.initialize_deque_ixn(payer, 0),
        ],
        "create base and quote mint ATAs for `payer`, then initialize the deque".to_string(),
    )
    .context("Should initialize")?;

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
    fuzz(rpc, payer, payer_base_ata, ctx, ROUNDS)?;

    Ok(())
}

pub fn fuzz(
    rpc: &RpcClient,
    payer: &Keypair,
    payer_base_ata: Pubkey,
    ctx: DequeContext,
    rounds: u64,
) -> anyhow::Result<()> {
    for round in 0..rounds {
        println!("---------------- Fuzz round: {} ----------------", round,);
        // Pseudo-random-ish deposits count in {1,2,3}
        let num_deposits = ((round * 7 + 3) % 3) + 1;

        let mut expected = 0;
        for j in 0..num_deposits {
            // Vary the deposit amount but keep it sane and non-zero
            let amount = 1_000 + ((round * 997) ^ (j * 313)) % (INITIAL_MINT_AMOUNT * rounds);
            expected += amount;

            send_deposit_or_withdraw(
                rpc,
                payer,
                ctx.deque_pubkey,
                payer_base_ata,
                ctx.base_mint,
                ctx.vault_base_ata,
                &DequeInstruction::Deposit(DepositInstructionData {
                    amount,
                    choice: MarketChoice::Base,
                }),
            )
            .context("Couldn't deposit base")?;
        }

        // Exactly one withdraw after ≥1 deposits
        send_deposit_or_withdraw(
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
        .context("Couldn't withdraw base")?;

        println!("Expected withdrawn: {}", expected);
    }

    print_size_and_sectors(rpc, &ctx.deque_pubkey);

    Ok(())
}
