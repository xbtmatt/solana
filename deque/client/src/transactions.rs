use anyhow::Context;
use deque::{instruction_enum::DequeInstruction, seeds};
use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    system_program,
    transaction::Transaction,
};

use crate::{tokens::get_token_balance, views::inspect_account};

pub async fn fund_account(rpc: &RpcClient, keypair: Option<Keypair>) -> anyhow::Result<Keypair> {
    let payer = match keypair {
        Some(kp) => kp,
        None => Keypair::new(),
    };

    let airdrop_signature = rpc
        .request_airdrop(&payer.pubkey(), 2_000_000_000)
        .context("Failed to request airdrop")?;

    let mut i = 0;
    // Wait for airdrop confirmation.
    while !rpc
        .confirm_transaction(&airdrop_signature)
        .context("Couldn't confirm transaction")?
        && i < 10
    {
        std::thread::sleep(std::time::Duration::from_millis(500));
        i += 1;
    }

    Ok(payer)
}

#[allow(clippy::result_large_err)]
pub fn send_txn(
    rpc: &RpcClient,
    payer: &Keypair,
    signers: &[&Keypair],
    ixns: Vec<Instruction>,
    txn_label: String,
) -> Result<Signature, ClientError> {
    let bh = rpc
        .get_latest_blockhash()
        .or(Err(()))
        .expect("Should be able to get blockhash.");
    let msg = Message::new(
        &[
            vec![
                ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
                ComputeBudgetInstruction::set_compute_unit_price(1),
            ],
            ixns,
        ]
        .concat(),
        Some(&payer.pubkey()),
    );
    let mut tx = Transaction::new_unsigned(msg);
    tx.try_sign(
        &[std::iter::once(payer)
            .chain(signers.iter().cloned())
            .collect::<Vec<_>>()]
        .concat(),
        bh,
    )
    .expect("Should sign");

    match rpc.send_and_confirm_transaction(&tx) {
        Ok(sig) => {
            println!("✓: Called {}, {}", txn_label, sig);
            Ok(sig)
        }
        Err(e) => {
            eprintln!("Failed to call {}: {}", txn_label, e);
            Err(e)
        }
    }
}

pub enum DepositOrWithdraw {
    Deposit,
    Withdraw,
}

#[allow(clippy::result_large_err)]
pub fn send_deposit_or_withdraw(
    rpc: &RpcClient,
    payer: &Keypair,
    deque_pubkey: Pubkey,
    payer_ata: Pubkey,
    mint: Pubkey,
    vault_ata: Pubkey,
    deque_instruction: &DequeInstruction,
) -> Result<ParsedTransaction, ClientError> {
    println!(
        "BEFORE: payer, vault: ({}, {})",
        get_token_balance(rpc, &payer.pubkey(), &mint),
        get_token_balance(rpc, &deque_pubkey, &mint)
    );

    let label = match deque_instruction {
        DequeInstruction::Deposit(_) => "deposit",
        DequeInstruction::Withdraw(_) => "withdraw",
        _ => panic!("Instruction must be deposit or withdraw."),
    };

    let ixn = Instruction {
        program_id: deque::ID,
        data: deque_instruction.pack(),
        accounts: vec![
            AccountMeta::new_readonly(deque::ID, false),
            AccountMeta::new_readonly(seeds::event_authority::ID, false),
            AccountMeta::new(deque_pubkey, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(payer_ata, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(vault_ata, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    };

    let sig = send_txn(rpc, payer, &[payer], vec![ixn], label.to_string())?;

    println!(
        "AFTER:  payer, vault: ({}, {})",
        get_token_balance(rpc, &payer.pubkey(), &mint),
        get_token_balance(rpc, &deque_pubkey, &mint)
    );

    inspect_account(rpc, &deque_pubkey, false);

    Ok(sig)
}
