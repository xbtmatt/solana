use anyhow::Context;
use deque::instruction_enum::MarketChoice;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    signature::{Keypair, Signature},
    signer::Signer,
};

use crate::{
    tokens::{DequeContext, INITIAL_MINT_AMOUNT},
    transactions::send_txn,
};

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
        vec![ctx.initialize_deque_ixn(payer, init_num_sectors)],
        "create base and quote mint ATAs for `payer`, then initialize the deque".to_string(),
    )
    .context("Should initialize")
}

pub fn init_atas_and_send_tokens_to_acc(
    funder: &Keypair,
    rpc: &RpcClient,
    receiver: &Keypair,
    ctx: &DequeContext,
    num_payers: u64,
) -> anyhow::Result<Signature> {
    let (funder_base_ata, funder_quote_ata) = ctx.get_atas(&funder.pubkey());
    let (receiver_base_ata, receiver_quote_ata) = ctx.get_atas(&receiver.pubkey());

    send_txn(
        rpc,
        receiver,
        &[funder],
        vec![
            ctx.create_ata_ixn(receiver, MarketChoice::Base),
            ctx.create_ata_ixn(receiver, MarketChoice::Quote),
            spl_token::instruction::transfer(
                &ctx.base_token_program,
                &funder_base_ata,
                &receiver_base_ata,
                &funder.pubkey(),
                &[],
                INITIAL_MINT_AMOUNT / num_payers,
            )?,
            spl_token::instruction::transfer(
                &ctx.quote_token_program,
                &funder_quote_ata,
                &receiver_quote_ata,
                &funder.pubkey(),
                &[],
                INITIAL_MINT_AMOUNT / num_payers,
            )?,
        ],
        format!("create base and quote for {}", receiver.pubkey()),
    )
}
