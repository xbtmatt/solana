use std::{os::raw, str::FromStr};

use anyhow::Context;
use deque::{events::DequeEvent, instruction_enum::InstructionTag};
use itertools::Itertools;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;

use crate::{
    ellipsis_transaction_utils::{
        parse_transaction, ParsedInnerInstruction, ParsedInstruction, ParsedTransaction,
    },
    logs::bytes_to_str,
};

pub fn parse_txn(
    rpc: &solana_client::rpc_client::RpcClient,
    sig: Signature,
) -> anyhow::Result<Vec<DequeEvent<'_>>> {
    let txn = rpc
        .get_transaction_with_config(
            &sig,
            solana_client::rpc_config::RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Json),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
            },
        )
        .context("Failed to get txn from rpc")?;

    let parsed = parse_transaction(txn);
    parse_events(&parsed)?;

    Ok(vec![])
}

pub fn parse_events(tx: &ParsedTransaction) -> anyhow::Result<Vec<u8>> {
    let _sig = Signature::from_str(&tx.signature)
        .ok()
        .context("Couldn't convert signature to string");

    let event_bytes = tx
        .inner_instructions
        .iter()
        .flat_map(|inner_ixns| {
            inner_ixns
                .iter()
                .filter_map(|ParsedInnerInstruction { instruction, .. }| {
                    maybe_deque_event_bytes(instruction)
                })
        })
        .flatten()
        .collect_vec();

    println!("event bytes: {:?}", event_bytes);
    println!("----------------------------------------------------------------------\n");

    // TODO: Iterate over the flattened vecs (or unflatten them? to parse them as Vec<Vec<u8>> separately)
    // and parse event data per the bytes.

    Ok(event_bytes)
}

pub fn maybe_deque_event_bytes(ixn: &ParsedInstruction) -> Option<Vec<u8>> {
    if ixn.data.is_empty() || ixn.program_id.as_str() != deque::id_str() {
        return None;
    };

    ixn.data.split_first().and_then(|(tag, data)| {
        InstructionTag::try_from(*tag).ok().and_then(|t| {
            if matches!(t, InstructionTag::FlushEventLog) {
                Some(data.to_vec())
            } else {
                None
            }
        })
    })
}
