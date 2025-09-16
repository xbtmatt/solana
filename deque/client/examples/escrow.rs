use deque::{
    state::{DequeInstruction, MarketEscrowChoice},
    PROGRAM_ID_PUBKEY,
};
use deque_client::{
    logs::print_size_and_sectors,
    tokens::{generate_deque, GeneratedDequeContext, INITIAL_MINT_AMOUNT},
    transactions::{fund_account, send_deposit_or_withdraw, send_txn},
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    system_program,
};

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
    let GeneratedDequeContext {
        base_mint,
        quote_mint,
        payer_base_ata,
        payer_quote_ata: _,
        deque_pubkey,
        vault_base_ata,
        vault_quote_ata,
    } = generate_deque(rpc, payer).expect("Should be able to generate deque");

    println!("deque pubkey {:#?}", deque_pubkey.to_string());
    println!("base mint pubkey {:#?}", base_mint.to_string());
    println!("quote mint pubkey {:#?}", quote_mint.to_string());
    println!("payer_base_ata {:#?}", payer_base_ata.to_string());

    // ------------------------------------- Initialization ----------------------------------------
    send_txn(
        rpc,
        payer,
        &[payer],
        vec![
            // Initialize the deque
            Instruction::new_with_borsh(
                PROGRAM_ID_PUBKEY,
                &DequeInstruction::Initialize { num_sectors: 0 },
                vec![
                    AccountMeta::new(deque_pubkey, false),
                    AccountMeta::new(payer.pubkey(), true),
                    AccountMeta::new_readonly(system_program::id(), false),
                    AccountMeta::new_readonly(spl_token::id(), false),
                    // For the ATA creation inside the program.
                    AccountMeta::new(vault_base_ata, false),
                    AccountMeta::new(vault_quote_ata, false),
                    AccountMeta::new_readonly(base_mint, false),
                    AccountMeta::new_readonly(quote_mint, false),
                    AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                ],
            ),
            // Then create the base mint ATA for the payer.
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &payer.pubkey(),
                &payer.pubkey(),
                &base_mint,
                &spl_token::id(),
            ),
        ],
        "create associated token account and initialize market deque".to_string(),
    );

    // ----------------------------------------- Deposit -------------------------------------------
    send_deposit_or_withdraw(
        rpc,
        payer,
        deque_pubkey,
        payer_base_ata,
        base_mint,
        vault_base_ata,
        &DequeInstruction::Deposit {
            amount: 1000,
            choice: MarketEscrowChoice::Base,
        },
    );

    // ----------------------------------------- Withdraw -------------------------------------------
    send_deposit_or_withdraw(
        rpc,
        payer,
        deque_pubkey,
        payer_base_ata,
        base_mint,
        vault_base_ata,
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
                deque_pubkey,
                payer_base_ata,
                base_mint,
                vault_base_ata,
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
            deque_pubkey,
            payer_base_ata,
            base_mint,
            vault_base_ata,
            &DequeInstruction::Withdraw {
                choice: MarketEscrowChoice::Base,
            },
        );

        println!("Expected withdrawn: {}", expected);
    }

    print_size_and_sectors(rpc, &deque_pubkey);
}
