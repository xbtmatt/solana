use anyhow::Context;
use deque::instruction_enum::{
    DepositInstructionData, DequeInstruction, MarketChoice, WithdrawInstructionData,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::{
    tokens::{DequeContext, INITIAL_MINT_AMOUNT},
    transactions::send_deposit_or_withdraw,
};

pub fn fuzz(
    rpc: &RpcClient,
    payer: &Keypair,
    ctx: DequeContext,
    rounds: u64,
) -> anyhow::Result<()> {
    let (payer_base_ata, _payer_quote_ata) = ctx.get_atas(&payer.pubkey());

    for round in 0..rounds {
        println!("---------------- Fuzz round: {} ----------------", round,);
        // Pseudo-random-ish deposits count in {1,2,3}
        let num_deposits = ((round * 7 + 3) % 3) + 1;

        let mut expected = 0;
        for j in 0..num_deposits {
            // Vary the deposit amount but keep it sane and non-zero
            let amount = 1_000 + ((round * 997) ^ (j * 313)) % (INITIAL_MINT_AMOUNT * rounds);
            expected += amount;

            send_deposit_or_withdraw(
                rpc,
                payer,
                ctx.deque_pubkey,
                payer_base_ata,
                ctx.base_mint,
                ctx.vault_base_ata,
                &DequeInstruction::Deposit(DepositInstructionData::new(amount, MarketChoice::Base)),
            )
            .context("Couldn't deposit base")?;
        }

        // Exactly one withdraw after ≥1 deposits
        send_deposit_or_withdraw(
            rpc,
            payer,
            ctx.deque_pubkey,
            payer_base_ata,
            ctx.base_mint,
            ctx.vault_base_ata,
            &DequeInstruction::Withdraw(WithdrawInstructionData::new(MarketChoice::Base)),
        )
        .context("Couldn't withdraw base")?;

        println!("Expected withdrawn: {}", expected);
    }

    Ok(())
}
