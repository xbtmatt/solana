use anyhow::Context;
use deque::{
    events::{
        DepositEventData, DequeEvent, EmittableEvent, EventTag, HeaderEventData, WithdrawEventData,
    },
    instruction_enum::InstructionTag,
};
use itertools::Itertools;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status::UiTransactionEncoding;

use crate::ellipsis_transaction_utils::{
    parse_transaction, ParsedInnerInstruction, ParsedTransaction,
};

impl ParsedTransaction {
    pub fn get_inner_deque_events(&self) -> anyhow::Result<Vec<DequeEvent<'_>>> {
        let events = self
            .inner_instructions
            .iter()
            .flatten()
            .filter_map(maybe_unpack_events)
            .flatten()
            .collect_vec();

        Ok(events)
    }
}

pub fn fetch_parsed_txn(
    rpc: &solana_client::rpc_client::RpcClient,
    sig: Signature,
) -> anyhow::Result<ParsedTransaction> {
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

    Ok(parse_transaction(txn))
}

pub fn maybe_unpack_events(inner_ixn: &ParsedInnerInstruction) -> Option<Vec<DequeEvent<'_>>> {
    let ixn = &inner_ixn.instruction;

    if ixn.program_id.as_str() != deque::id_str() {
        return None;
    };

    let (tag, data) = ixn.data.split_first()?;
    let instruction_tag = InstructionTag::try_from(*tag).ok()?;

    matches!(instruction_tag, InstructionTag::FlushEventLog)
        .then(|| ixn_bytes_to_events(data).ok())
        .flatten()
}

/// Unpacks a slab of bytes into deque events.
/// Note that the data here is expected to start at the *first* byte after the ixn tag discriminant.
/// That is, `all_data` should start  the *event* tag/discriminant.
pub fn ixn_bytes_to_events(all_data: &[u8]) -> anyhow::Result<Vec<DequeEvent<'_>>> {
    let mut i = 0;
    let mut res = vec![];
    loop {
        let data = &all_data[i..];
        if data.is_empty() {
            break;
        }

        let event_tag = EventTag::try_from(data[0])?;
        let (event, len) = match event_tag {
            EventTag::Header => (
                DequeEvent::Header(HeaderEventData::try_from_slice(data)?),
                HeaderEventData::LEN,
            ),
            EventTag::Initialize => todo!(),
            EventTag::Deposit => (
                DequeEvent::Deposit(DepositEventData::try_from_slice(data)?),
                DepositEventData::LEN,
            ),
            EventTag::Withdraw => (
                DequeEvent::Withdraw(WithdrawEventData::try_from_slice(data)?),
                WithdrawEventData::LEN,
            ),
            EventTag::Resize => todo!(),
        };

        i += len;
        res.push(event);
    }

    Ok(res)
}

#[test]
fn test_multiple_events_in_slab() {
    use deque::instruction_enum::MarketChoice;
    use solana_sdk::signature::{Keypair, Signer};
    use solana_sdk::syscalls::MAX_CPI_INSTRUCTION_DATA_LEN;

    let (trader_1, trader_2, market) = (
        Keypair::new().pubkey(),
        Keypair::new().pubkey(),
        Keypair::new().pubkey(),
    );
    let (amount_1, amount_2) = (10000, 20000);
    let header = HeaderEventData::new(
        InstructionTag::Resize,
        &market,
        &trader_1,
        u64::MAX - 1,
        u16::MAX - 1,
    );
    let deposit_1 = DepositEventData::new(&trader_1, amount_1, MarketChoice::Base);
    let deposit_2 = DepositEventData::new(&trader_2, amount_2, MarketChoice::Quote);
    let deposit_3 = DepositEventData::new(&trader_2, amount_2, MarketChoice::Quote);
    let withdraw_1 = WithdrawEventData::new(&trader_2, amount_2, MarketChoice::Quote);
    let withdraw_2 = WithdrawEventData::new(&trader_2, amount_2, MarketChoice::Quote);

    let events = [
        DequeEvent::Header(header),
        DequeEvent::Deposit(deposit_1),
        DequeEvent::Deposit(deposit_2),
        DequeEvent::Deposit(deposit_3),
        DequeEvent::Withdraw(withdraw_1),
        DequeEvent::Withdraw(withdraw_2),
    ];

    let mut buf: Vec<u8> = Vec::with_capacity(MAX_CPI_INSTRUCTION_DATA_LEN as usize);

    for event in events.iter() {
        match event {
            DequeEvent::Header(header) => header.write(&mut buf).expect("Should write"),
            DequeEvent::Deposit(deposit) => deposit.write(&mut buf).expect("Should write"),
            DequeEvent::Withdraw(withdraw) => withdraw.write(&mut buf).expect("Should write"),
        };
    }

    let parsed_events = ixn_bytes_to_events(&buf[..]).expect("Should parse all");
    assert_eq!(parsed_events.len(), events.len());

    events
        .into_iter()
        .zip(parsed_events)
        .for_each(|(e1, e2)| assert_eq!(e1, e2));
}
