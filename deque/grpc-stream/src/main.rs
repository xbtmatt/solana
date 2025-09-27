use deque::{instruction_enum::InstructionTag, seeds::event_authority};
use deque_client::events::maybe_unpack_event_bytes_with_tag;
use futures::StreamExt;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{geyser::subscribe_update::UpdateOneof, prelude::*};

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
                    match update {
                        UpdateOneof::Account(acc) => {
                            if let Some(account) = acc.account {
                                println!("Account data: {:?}", &account.data[0..100]);
                            }
                        }
                        UpdateOneof::Transaction(txn) => {
                            if let Some(txn) = txn.transaction {
                                txn.meta.inspect(|meta| {
                                    for inner_instructions in meta.inner_instructions.iter() {
                                        for instruction in inner_instructions.instructions.iter() {
                                            // TODO: Need to match this up with the program ID so we can check easily.
                                            // then we can do: if instruction.program_id_index...
                                            if let Some(events) =
                                                maybe_unpack_event_bytes_with_tag(&instruction.data)
                                            {
                                                println!("{:#?}", events);
                                            }
                                        }
                                    }
                                });

                                txn.transaction.inspect(|txn| {
                                    txn.message.as_ref().inspect(|msg| {
                                        println!(
                                            "compiled txn instructions {:#?}",
                                            msg.instructions
                                        )
                                    });
                                });
                            }
                        }
                        _ => (),
                    }
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
