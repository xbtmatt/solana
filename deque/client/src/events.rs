use std::str::FromStr;

use anyhow::Context;
use deque::{events::DequeEvent, instruction_enum::InstructionTag};
use itertools::Itertools;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;

use crate::ellipsis_transaction_utils::{
    parse_transaction, ParsedInnerInstruction, ParsedTransaction,
};

pub fn get_transaction_events(
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

pub fn parse_events(tx: &ParsedTransaction) -> anyhow::Result<Vec<DequeEvent<'_>>> {
    let _sig = Signature::from_str(&tx.signature)
        .ok()
        .context("Couldn't convert signature to string");

    let events = tx
        .inner_instructions
        .iter()
        .flatten()
        .filter_map(maybe_unpack_event)
        .collect_vec();

    Ok(events)
}

pub fn maybe_unpack_event(inner_ixn: &ParsedInnerInstruction) -> Option<DequeEvent<'_>> {
    let ixn = &inner_ixn.instruction;

    if ixn.program_id.as_str() != deque::id_str() {
        return None;
    };

    let (tag, data) = ixn.data.split_first()?;
    let instruction_tag = InstructionTag::try_from(*tag).ok()?;

    matches!(instruction_tag, InstructionTag::FlushEventLog)
        .then(|| DequeEvent::unpack(data).ok())
        .flatten()
}
