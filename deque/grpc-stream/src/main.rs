use deque::seeds::event_authority;
use futures::StreamExt;
use grpc_stream::parse_update::parse_update;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let endpoint = "http://localhost:10000";

    let mut client = GeyserGrpcClient::build_from_static(endpoint)
        .connect()
        .await?;

    let mut stream = client
        .subscribe_once(SubscribeRequest {
            accounts: HashMap::from([(
                "event authority pda account data".to_string(),
                SubscribeRequestFilterAccounts {
                    account: vec![event_authority::ID.to_string()],
                    owner: vec![],
                    filters: vec![],
                    nonempty_txn_signature: Some(true),
                },
            )]),
            slots: HashMap::new(),
            transactions: HashMap::from([(
                "event authority pda ixn data".to_string(),
                SubscribeRequestFilterTransactions {
                    failed: None,
                    signature: None,
                    vote: None,
                    account_exclude: vec![],
                    account_include: vec![],
                    account_required: vec![event_authority::ID.to_string()],
                },
            )]),
            transactions_status: HashMap::new(),
            blocks: HashMap::new(),
            entry: HashMap::new(),
            blocks_meta: HashMap::new(),
            commitment: Some(CommitmentLevel::Processed.into()),
            accounts_data_slice: vec![],
            ping: None,
            from_slot: None,
        })
        .await?;

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => {
                if let Some(update) = msg.update_oneof {
                    parse_update(update);
                }
            }
            Err(error) => {
                eprintln!("❌ Stream error: {}", error);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Ok(())
}
