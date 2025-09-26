use std::sync::Arc;

use anyhow::{Context, Result};
use deque::{
    state::{
        Deque, EphemeralEventHeader, DEQUE_ACCOUNT_DISCRIMINANT, EPHEMERAL_EVENT_LOG_HEADER_SIZE,
        EVENT_ACCOUNT_DISCRIMINANT,
    },
    utils::from_slab_bytes_mut,
};
use futures::future::join_all;
use itertools::Itertools;
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer};
use tokio_stream::StreamExt;

use crate::{
    events::unpack_event_bytes,
    fuzz::fuzz,
    initialize::{init_atas_and_send_tokens_to_acc, initialize_market_and_event_authority},
    tokens::{generate_market, MarketContext},
    transactions::fund_account,
};

const RPC_URL: &str = "http://localhost:8899";

pub async fn subscribe_program_and_send() -> Result<()> {
    let ws_client = PubsubClient::new("ws://localhost:8900").await?;

    // ------------------------- DEQUE PROGRAM WS DATA ------------------------
    let program_subscription = tokio::spawn(async move {
        let (mut stream, _) = ws_client
            .program_subscribe(
                &deque::id(),
                // Not really necessary if our program only owns the PDAs relevant to the contract,
                // but this is for how you'd filter on specific account types owned by the program.
                Some(RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                        0,
                        DEQUE_ACCOUNT_DISCRIMINANT.to_vec(),
                    ))]),
                    account_config: RpcAccountInfoConfig {
                        commitment: Some(CommitmentConfig::confirmed()),
                        encoding: Some(UiAccountEncoding::Base64),
                        data_slice: None,
                        min_context_slot: None,
                    },
                    with_context: Some(true),
                    sort_results: Some(true),
                }),
            )
            .await
            .expect("Should be able to subscribe to the program account updates");

        while let Some(account) = stream.next().await {
            // Technically this could be the `_unchecked` version of this call since we filtered by
            // the discriminant already, but it's a simple extra check.
            account.value.account.data.decode().inspect(|bytes| {
                if let Ok(deque) = Deque::from_bytes(&mut bytes.to_owned()) {
                    println!("{deque:#?}");
                } else {
                    println!("Failed to unpack deque account.");
                }
            });
        }
    });

    // ------------------------- EVENT AUTHORITY ACCOUNT WS DATA ------------------------
    let ws_client = PubsubClient::new("ws://localhost:8900").await?;
    let event_authority_account_subscription = tokio::spawn(async move {
        let (mut stream, _) = ws_client
            .program_subscribe(
                &deque::id(),
                // Not really necessary if our program only owns the PDAs relevant to the contract,
                // but this is for how you'd filter on specific account types owned by the program.
                Some(RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                        0,
                        EVENT_ACCOUNT_DISCRIMINANT.to_vec(),
                    ))]),
                    account_config: RpcAccountInfoConfig {
                        commitment: Some(CommitmentConfig::confirmed()),
                        encoding: Some(UiAccountEncoding::Base64),
                        data_slice: None,
                        min_context_slot: None,
                    },
                    with_context: Some(true),
                    sort_results: Some(true),
                }),
            )
            .await
            .expect("Should be able to subscribe to the program account updates");

        while let Some(account) = stream.next().await {
            // Technically this could be the `_unchecked` version of this call since we filtered by
            // the discriminant already, but it's a simple extra check.
            if let Some(mut bytes) = account.value.account.data.decode() {
                let (header_bytes, ref rest) = bytes.split_at_mut(EPHEMERAL_EVENT_LOG_HEADER_SIZE);

                let header = from_slab_bytes_mut::<EphemeralEventHeader>(header_bytes, 0_usize)
                    .expect("Should be able to unpack header bytes into ephemeral log header");

                let (event_bytes, _garbage) = rest.split_at(header.get_written_bytes() as usize);

                let events = unpack_event_bytes(event_bytes)
                    .expect("Should be able to unpack event bytes in event account data");

                println!("events: {events:#?}");
            }
        }
    });

    let rpc = create_client();
    let primary_payer = fund_account(&rpc, None).await.expect("Should fund account");
    let ctx = generate_market(&rpc, &primary_payer).expect("Should be able to generate deque");
    initialize_market_and_event_authority(&rpc, &primary_payer, &ctx)
        .expect("Should initialize the deque");

    // Fund all payers.
    let num_payers = 10;
    let payers =
        create_and_fund_payers(primary_payer.to_base58_string().as_str(), &ctx, num_payers).await;

    // Then spawn fuzz tests for each of them concurrently.
    let payer_fuzzes: Vec<_> = payers
        .into_iter()
        .map(|payer| {
            let rpc = Arc::new(create_client());
            let ctx = ctx.clone();
            tokio::spawn(async move {
                let err_msg = format!("Fuzz test failed for payer: {}", payer.pubkey());
                fuzz(&rpc, &payer, &ctx, 10, Some(num_payers)).context(err_msg)
            })
        })
        .collect();

    tokio::select! {
        result1 = program_subscription => {
            println!("Program subscription errored out!, {result1:?}");
        },
        result2 = join_all(payer_fuzzes) => {
            println!("Market escrow complete! {result2:?}");
        },
        result3 = event_authority_account_subscription => {
            println!("Event authority account subscription errored out!, {result3:?}");
        }
    }

    Ok(())
}

async fn create_and_fund_payers(
    funder_kp_str: &str,
    ctx: &MarketContext,
    num_payers: usize,
) -> Vec<Keypair> {
    let rpc = Arc::new(create_client());
    let ctx = ctx.clone();

    let tasks: Vec<_> = (0..num_payers)
        .map(|_| {
            let rpc = rpc.clone();
            let ctx = ctx.clone();
            let funder = Keypair::from_base58_string(funder_kp_str);
            tokio::spawn(async move {
                let payer = fund_account(&rpc, None).await.expect("Should fund account");
                init_atas_and_send_tokens_to_acc(&funder, &rpc, &payer, &ctx, num_payers as u64)
                    .map(|_| payer)
            })
        })
        .collect();

    join_all(tasks)
        .await
        .into_iter()
        .flatten()
        .filter_map(|result| result.ok())
        .collect_vec()
}

fn create_client() -> RpcClient {
    RpcClient::new_with_commitment(RPC_URL.to_string(), CommitmentConfig::confirmed())
}
