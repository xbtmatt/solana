use deque::{
    state::{
        get_deque_address, Deque, DequeInstruction, DequeNode, MarketEscrow, MarketEscrowChoice,
        HEADER_FIXED_SIZE,
    },
    utils::{from_sector_idx, SECTOR_SIZE},
    PROGRAM_ID_PUBKEY,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    message::Message,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction, system_program,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Mint;

#[tokio::main]
async fn main() {
    // Connect to local cluster
    let rpc_url = String::from("http://localhost:8899");
    let client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Generate a new keypair for paying fees
    let payer = Keypair::new();

    // Request airdrop of 2 SOL for transaction fees
    println!("Requesting airdrop...");
    let airdrop_signature = client
        .request_airdrop(&payer.pubkey(), 2_000_000_000)
        .expect("Failed to request airdrop");

    // Wait for airdrop confirmation
    loop {
        if client
            .confirm_transaction(&airdrop_signature)
            .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    println!("Airdrop confirmed\n");

    println!("=== Testing market escrow ===");
    test_market_escrow(&client, &payer);
}

fn test_market_escrow(rpc: &RpcClient, payer: &Keypair) {
    const INITIAL_AMOUNT: u64 = 100000;
    // ------------------------------------ Mint two tokens ----------------------------------------
    let (base_mint, _payer_base_ata) =
        create_token(rpc, payer, 10, INITIAL_AMOUNT).expect("Should mint.");
    let (quote_mint, _payer_quote_ata) =
        create_token(rpc, payer, 10, INITIAL_AMOUNT).expect("Should mint.");
    let (deque_pubkey, _deque_bump) = get_deque_address(&base_mint, &quote_mint);

    println!("deque pubkey {:#?}", deque_pubkey.to_string());
    println!("base mint pubkey {:#?}", base_mint.to_string());
    println!("quote mint pubkey {:#?}", quote_mint.to_string());

    // ------------------------------------- Initialization ----------------------------------------
    let init_data = borsh::to_vec(&DequeInstruction::Initialize { num_sectors: 0 })
        .expect("Failed to serialize");

    let (vault_base_ata, vault_quote_ata) = (
        get_associated_token_address(&deque_pubkey, &base_mint),
        get_associated_token_address(&deque_pubkey, &quote_mint),
    );

    let init_instruction = Instruction::new_with_bytes(
        PROGRAM_ID_PUBKEY,
        &init_data,
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
    );

    send_txn(
        rpc,
        payer,
        &[payer],
        vec![init_instruction],
        "initialize Market Deque".to_string(),
    );

    let create_ata_ixn =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &payer.pubkey(),
            &payer.pubkey(),
            &base_mint,
            &spl_token::id(),
        );

    send_txn(
        rpc,
        payer,
        &[payer],
        vec![create_ata_ixn],
        "create associated token account".to_string(),
    );

    let payer_ata = get_associated_token_address(&payer.pubkey(), &base_mint);
    println!("payer_ata {:#?}", payer_ata.to_string());

    // ----------------------------------------- Deposit -------------------------------------------
    send_deposit_or_withdraw(
        rpc,
        payer,
        deque_pubkey,
        payer_ata,
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
        payer_ata,
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
            let amount = 1_000 + ((round * 997) ^ (j * 313)) % (INITIAL_AMOUNT * ROUNDS);
            expected += amount;

            send_deposit_or_withdraw(
                rpc,
                payer,
                deque_pubkey,
                payer_ata,
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
            payer_ata,
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

fn print_size_and_sectors(client: &RpcClient, account_pubkey: &Pubkey) {
    if let Ok(account) = client.get_account(account_pubkey) {
        let len = account.data.len();
        println!(
            "\nAccount size: {} bytes, {} sectors\n",
            len,
            (len - HEADER_FIXED_SIZE) / SECTOR_SIZE
        );
    }
}

fn inspect_account(client: &RpcClient, account_pubkey: &Pubkey, verbose: bool) {
    match client.get_account(account_pubkey) {
        Ok(account) => {
            if verbose {
                println!("Account owner: {}", account.owner);
                println!("Account lamports: {}", account.lamports);
                println!("Account data length: {} bytes", account.data.len());
                println!("Account executable: {}", account.executable);

                // Display raw bytes (first 100 or so)
                println!("\nRaw data (hex):");
                let display_len = std::cmp::min(account.data.len(), 100);
                for (i, chunk) in account.data[..display_len].chunks(16).enumerate() {
                    print!("{:04}: ", i * 16);
                    for byte in chunk {
                        print!("{:02x} ", byte);
                    }
                    println!();
                }
            }

            let cloned_data = &mut account.data.clone();
            let deque =
                Deque::new_from_bytes(cloned_data).expect("Should be able to cast directly.");
            if verbose {
                println!(
                    "len: {}, deque_head: {:#?}, deque_tail: {:#?}, free_head: {:#?}",
                    deque.header.len,
                    deque.header.deque_head,
                    deque.header.deque_tail,
                    deque.header.free_head,
                );
            }

            let from_head = deque
                .iter_indices::<MarketEscrow>()
                .map(|it| {
                    *from_sector_idx::<DequeNode<MarketEscrow>>(deque.sectors, it)
                        .expect("Should be valid.")
                })
                .collect::<Vec<DequeNode<MarketEscrow>>>();
            println!(
                "{:?}",
                from_head.iter().map(|f| f.inner).collect::<Vec<_>>()
            );
        }
        Err(e) => {
            println!("Failed to get account: {}", e);
        }
    }
}

// Generic transaction.
pub fn send_txn(
    rpc: &RpcClient,
    payer: &Keypair,
    signers: &[&Keypair],
    ixns: Vec<Instruction>,
    txn_label: String,
) {
    let bh = rpc
        .get_latest_blockhash()
        .or(Err(()))
        .expect("Should be able to get blockhash.");
    let msg = Message::new(&ixns, Some(&payer.pubkey()));
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
        }
        Err(e) => {
            eprintln!("Failed to call {}: {}", txn_label, e);
            panic!();
        }
    };
}

/// Returns the mint pubkey and the token account pubkey.
fn create_token(
    rpc: &RpcClient,
    payer: &Keypair,
    mint_decimals: u8,
    mint_amt: u64,
) -> Result<(Pubkey, Pubkey), ()> {
    let mint = Keypair::new();
    let mint_rent = rpc
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .or(Err(()))?;
    let create_mint = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        Mint::LEN as u64,
        &spl_token::id(),
    );
    let init_mint = spl_token::instruction::initialize_mint2(
        &spl_token::id(),
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        mint_decimals,
    )
    .or(Err(()))?;

    let payer_ata = get_associated_token_address(&payer.pubkey(), &mint.pubkey());
    let create_ata =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &payer.pubkey(),
            &payer.pubkey(),
            &mint.pubkey(),
            &spl_token::id(),
        );

    let mint_to = spl_token::instruction::mint_to_checked(
        &spl_token::id(),
        &mint.pubkey(),
        &payer_ata,
        &payer.pubkey(),
        &[],
        mint_amt,
        mint_decimals,
    )
    .or(Err(()))?;

    send_txn(
        rpc,
        payer,
        &[&mint],
        vec![create_mint, init_mint],
        "--- create and initialize mint ---".to_string(),
    );
    send_txn(
        rpc,
        payer,
        &[payer],
        vec![create_ata, mint_to],
        "--- create ATA and mint to it".to_string(),
    );

    Ok((mint.pubkey(), payer_ata))
}

pub fn get_token_balance(rpc: &RpcClient, owner: &Pubkey, mint: &Pubkey) -> u64 {
    let ata = get_associated_token_address(owner, mint);
    let acc_data = rpc
        .get_account(&ata)
        .expect("Should be able to get account")
        .data;
    let token_account =
        spl_token::state::Account::unpack(&acc_data).expect("Should have account data.");

    token_account.amount
}

pub enum DepositOrWithdraw {
    Deposit,
    Withdraw,
}

pub fn send_deposit_or_withdraw(
    rpc: &RpcClient,
    payer: &Keypair,
    deque_pubkey: Pubkey,
    payer_ata: Pubkey,
    mint: Pubkey,
    vault_ata: Pubkey,
    deque_instruction: &DequeInstruction,
) {
    println!(
        "BEFORE: payer, vault: ({}, {})",
        get_token_balance(rpc, &payer.pubkey(), &mint),
        get_token_balance(rpc, &deque_pubkey, &mint)
    );

    let label = match deque_instruction {
        DequeInstruction::Deposit {
            amount: _,
            choice: _,
        } => "deposit",
        DequeInstruction::Withdraw { choice: _ } => "withdraw",
        _ => panic!("Instruction must be deposit or withdraw."),
    };

    let ixn = Instruction::new_with_borsh(
        PROGRAM_ID_PUBKEY,
        deque_instruction,
        vec![
            AccountMeta::new(deque_pubkey, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(payer_ata, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(vault_ata, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    send_txn(rpc, payer, &[payer], vec![ixn], label.to_string());

    println!(
        "AFTER:  payer, vault: ({}, {})",
        get_token_balance(rpc, &payer.pubkey(), &mint),
        get_token_balance(rpc, &deque_pubkey, &mint)
    );

    inspect_account(rpc, &deque_pubkey, false);
}
