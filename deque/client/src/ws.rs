use anyhow::Result;
use deque::state::{Deque, DEQUE_ACCOUNT_DISCRIMINANT};
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::commitment_config::CommitmentConfig;
use tokio_stream::StreamExt;

use crate::{
    fuzz::fuzz, initialize::initialize_deque_with_ctx, tokens::generate_market,
    transactions::fund_account,
};

const RPC_URL: &str = "http://localhost:8899";

pub async fn subscribe_program_and_send() -> Result<()> {
    let ws_client = PubsubClient::new("ws://localhost:8900").await?;

    let program_subscription = tokio::spawn(async move {
        let (mut stream, _) = ws_client
            .program_subscribe(
                &deque::id(),
                // Not really necessary if our program only owns the PDAs relevant to the contract,
                // but this is for how you'd filter on specific account types owned by the program.
                Some(RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
                        0,
                        DEQUE_ACCOUNT_DISCRIMINANT.to_le_bytes().to_vec(),
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

            // let mut data = account.value.account.data.decode();
            // if let Some(bytes) = data {
            //     let deque =
            //         Deque::new_from_bytes(&mut bytes.to_owned()).expect("Should convert to deque!");
            //     println!()
            // }
            account.value.account.data.decode().inspect(|bytes| {
                if let Ok(deque) = Deque::new_from_bytes(&mut bytes.to_owned()) {
                    println!("{deque:#?}");
                };
            });
            // .inspect(|mut bytes| println!("{:?}", Deque::new_from_bytes(&mut bytes)));
        }
    });

    let escrow_example = tokio::spawn(async move {
        let rpc =
            RpcClient::new_with_commitment(RPC_URL.to_string(), CommitmentConfig::confirmed());

        let payer = fund_account(&rpc, None).await.expect("Should fund account");
        let ctx = generate_market(&rpc, &payer).expect("Should be able to generate deque");
        initialize_deque_with_ctx(&rpc, &payer, &ctx).expect("Should initialize the deque");
        fuzz(&rpc, &payer, ctx, 10).expect("Should run fuzz");
    });

    tokio::select! {
        result1 = program_subscription => {
            println!("Program subscription errored out!, {result1:?}");
        },
        result2 = escrow_example => {
            println!("Market escrow complete! {result2:?}");
        },
    }

    Ok(())
}
