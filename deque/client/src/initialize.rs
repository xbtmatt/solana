use anyhow::Context;
use deque::instruction_enum::MarketChoice;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::{Keypair, Signature};

use crate::{tokens::DequeContext, transactions::send_txn};

/// Create both payer ATAs and initialize the deque.
pub fn initialize_deque_with_ctx(
    rpc: &RpcClient,
    payer: &Keypair,
    ctx: &DequeContext,
) -> anyhow::Result<Signature> {
    let init_num_sectors = 0;

    send_txn(
        rpc,
        payer,
        &[payer],
        vec![
            ctx.create_ata_ixn(payer, MarketChoice::Base),
            ctx.create_ata_ixn(payer, MarketChoice::Quote),
            ctx.initialize_deque_ixn(payer, init_num_sectors),
        ],
        "create base and quote mint ATAs for `payer`, then initialize the deque".to_string(),
    )
    .context("Should initialize")
}
