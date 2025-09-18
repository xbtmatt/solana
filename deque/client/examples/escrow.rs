use deque::instruction_enum::{DequeInstruction, MarketEscrowChoice};
use deque_client::{
    logs::print_size_and_sectors,
    tokens::{generate_deque, INITIAL_MINT_AMOUNT},
    transactions::{fund_account, send_deposit_or_withdraw, send_txn},
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address;

#[tokio::main]
async fn main() {
    // Connect to local cluster
    let rpc_url = String::from("http://localhost:8899");
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let payer = fund_account(&rpc, None).await.expect("Should fund account");
    test_market_escrow(&rpc, &payer);
}

fn test_market_escrow(rpc: &RpcClient, payer: &Keypair) {
    // ----------------------- Mint two tokens and generate deque address --------------------------
    let ctx = generate_deque(rpc, payer).expect("Should be able to generate deque");
    let payer_base_ata = get_associated_token_address(&payer.pubkey(), &ctx.base_mint);
    let _payer_quote_ata = get_associated_token_address(&payer.pubkey(), &ctx.quote_mint);

    println!("deque pubkey {:#?}", ctx.deque_pubkey.to_string());
    println!("base mint pubkey {:#?}", ctx.base_mint.to_string());
    println!("quote mint pubkey {:#?}", ctx.quote_mint.to_string());
    println!("payer_base_ata {:#?}", payer_base_ata.to_string());

    // ------------------------------------- Initialization ----------------------------------------
    // Create both payer ATAs.
    send_txn(
        rpc,
        payer,
        &[payer],
        vec![
            ctx.create_ata_ixn(payer, MarketEscrowChoice::Base),
            ctx.create_ata_ixn(payer, MarketEscrowChoice::Quote),
            ctx.initialize_deque_ixn(payer, 0),
        ],
        "create base and quote mint ATAs for `payer`, then initialize the deque".to_string(),
    );

    // ----------------------------------------- Deposit -------------------------------------------
    send_deposit_or_withdraw(
        rpc,
        payer,
        ctx.deque_pubkey,
        payer_base_ata,
        ctx.base_mint,
        ctx.vault_base_ata,
        &DequeInstruction::Deposit {
            amount: 1000,
            choice: MarketEscrowChoice::Base,
        },
    );

    // ----------------------------------------- Withdraw -------------------------------------------
    send_deposit_or_withdraw(
        rpc,
        payer,
        ctx.deque_pubkey,
        payer_base_ata,
        ctx.base_mint,
        ctx.vault_base_ata,
        &DequeInstruction::Withdraw {
            choice: MarketEscrowChoice::Base,
        },
    );

    // ------------------------------------------- Fuzz --------------------------------------------
    const ROUNDS: u64 = 10;

    for round in 0..ROUNDS {
        println!("---------------- Fuzz round: {} ----------------", round,);
        // Pseudo-random-ish deposits count in {1,2,3}
        let num_deposits = ((round * 7 + 3) % 3) + 1;

        let mut expected = 0;
        for j in 0..num_deposits {
            // Vary the deposit amount but keep it sane and non-zero
            let amount = 1_000 + ((round * 997) ^ (j * 313)) % (INITIAL_MINT_AMOUNT * ROUNDS);
            expected += amount;

            send_deposit_or_withdraw(
                rpc,
                payer,
                ctx.deque_pubkey,
                payer_base_ata,
                ctx.base_mint,
                ctx.vault_base_ata,
                &DequeInstruction::Deposit {
                    amount,
                    choice: MarketEscrowChoice::Base,
                },
            );
        }

        // Exactly one withdraw after ≥1 deposits
        send_deposit_or_withdraw(
            rpc,
            payer,
            ctx.deque_pubkey,
            payer_base_ata,
            ctx.base_mint,
            ctx.vault_base_ata,
            &DequeInstruction::Withdraw {
                choice: MarketEscrowChoice::Base,
            },
        );

        println!("Expected withdrawn: {}", expected);
    }

    print_size_and_sectors(rpc, &ctx.deque_pubkey);
}
