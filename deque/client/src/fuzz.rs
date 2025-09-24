use anyhow::Context;
use deque::instruction_enum::{
    DepositInstructionData, DequeInstruction, MarketChoice, WithdrawInstructionData,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::{
    tokens::{DequeContext, INITIAL_MINT_AMOUNT},
    transactions::send_deposit_or_withdraw,
};

/// Run a randomized deposit/withdraw fuzz test against the deque program.
///
/// This function simulates a trader interacting with the on-chain deque by
/// performing a sequence of deposits and withdrawals of the base token.
/// It maintains a local model of the trader’s wallet balance and escrow
/// balance to ensure that no invalid transactions are ever sent:
/// - Deposits are only attempted when the wallet has available tokens.
/// - Withdrawals are only attempted when there are tokens in escrow.
/// - Withdrawals always empty the entire escrow (matching program semantics).
///
/// Each fuzz round randomly chooses one of three actions:
/// - **Deposit:** transfer a random, valid amount from wallet → escrow.
/// - **Withdraw:** withdraw the full escrow balance back to wallet.
/// - **Skip:** take no action this round.
///
/// The sequence of actions (e.g. `D, D, W, D, W, W`) is randomized using
/// a seeded RNG, making runs reproducible per payer pubkey while still
/// providing varied coverage across rounds. Balances are tracked locally,
/// so deposits and withdrawals are always valid and the fuzzer never fails
/// due to insufficient funds or empty escrow.
///
/// # Arguments
/// * `rpc` – Solana RPC client used to submit transactions.
/// * `payer` – Keypair of the trader executing deposits/withdrawals.
/// * `ctx` – Context object providing program/mint/vault addresses.
/// * `rounds` – Number of fuzz rounds to execute.
/// * `num_payers` – Optional number of concurrent payers to divide the
///   initial mint amount between (ensures fair balance distribution).
///
/// # Errors
/// Returns an error if any underlying deposit or withdraw transaction
/// submission fails.
pub fn fuzz(
    rpc: &RpcClient,
    payer: &Keypair,
    ctx: &DequeContext,
    rounds: u64,
    num_payers: Option<usize>,
) -> anyhow::Result<()> {
    let (payer_base_ata, _payer_quote_ata) = ctx.get_atas(&payer.pubkey());

    let per_fuzzer_mint = INITIAL_MINT_AMOUNT / (num_payers.unwrap_or(1) as u64);

    // Local model of balances
    let mut wallet_base: u64 = per_fuzzer_mint;
    let mut escrow_base: u64 = 0;

    // Deterministic randomness per payer
    let mut seed = [0u8; 32];
    let pk = payer.pubkey().to_bytes();
    for (i, b) in pk.iter().enumerate() {
        seed[i % 32] ^= *b;
    }
    seed[0] ^= (rounds as u8).wrapping_mul(31);
    let mut rng = SmallRng::from_seed(seed);

    for round in 0..rounds {
        println!("---------------- Fuzz round: {} ----------------", round);

        // Decide action: 0 = deposit, 1 = withdraw, 2 = skip
        let action = rng.random_range(0..=2);

        match action {
            0 => {
                if wallet_base > 0 {
                    // Random deposit amount, capped so it's nonzero and sane
                    let max_amt = wallet_base.min(per_fuzzer_mint / 4).max(1);
                    let amount = rng.random_range(1..=max_amt);

                    send_deposit_or_withdraw(
                        rpc,
                        payer,
                        ctx.deque_pubkey,
                        payer_base_ata,
                        ctx.base_mint,
                        ctx.vault_base_ata,
                        &DequeInstruction::Deposit(DepositInstructionData::new(
                            amount,
                            MarketChoice::Base,
                        )),
                    )
                    .with_context(|| format!("Couldn't deposit base (amt={amount})"))?;

                    wallet_base -= amount;
                    escrow_base += amount;
                    println!(
                        "Deposited {} (wallet={}, escrow={})",
                        amount, wallet_base, escrow_base
                    );
                } else {
                    println!("Wanted to deposit, but wallet is empty. Skipping.");
                }
            }
            1 => {
                if escrow_base > 0 {
                    send_deposit_or_withdraw(
                        rpc,
                        payer,
                        ctx.deque_pubkey,
                        payer_base_ata,
                        ctx.base_mint,
                        ctx.vault_base_ata,
                        &DequeInstruction::Withdraw(WithdrawInstructionData::new(
                            MarketChoice::Base,
                        )),
                    )
                    .context(format!("Couldn't withdraw base for: {:?}", &payer.pubkey()))?;

                    wallet_base += escrow_base;
                    println!(
                        "Withdrew {} (wallet={}, escrow=0)",
                        escrow_base, wallet_base
                    );
                    escrow_base = 0;
                } else {
                    println!("Wanted to withdraw, but escrow is empty. Skipping.");
                }
            }
            _ => {
                println!(
                    "Skipping action this round. (wallet={}, escrow={})",
                    wallet_base, escrow_base
                );
            }
        }
    }

    Ok(())
}
