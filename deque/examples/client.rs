use deque::{
    state::{
        get_deque_address, get_vault_address, Deque, DequeInstruction, DequeNode, DequeType,
        MarketEscrow, MarketEscrowChoice, HEADER_FIXED_SIZE,
    },
    utils::from_sector_idx,
    PROGRAM_ID_PUBKEY,
};
use solana_client::rpc_client::RpcClient;
use solana_program::example_mocks::solana_sdk::system_instruction;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    message::Message,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Mint;

#[tokio::main]
async fn main() {
    let program_id = PROGRAM_ID_PUBKEY;

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

    // println!("=== Testing Deque<u64, 5> ===");
    // test_u64_deque(&client, &payer, program_id);

    println!("=== Testing market escrow ===");
    test_market_escrow(&client, &payer, program_id);
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

fn test_u64_deque(rpc: &RpcClient, payer: &Keypair, program_id: Pubkey) {
    // ------------------------------------ Mint two tokens ----------------------------------------
    let (base_mint, _base_ata) = create_token(rpc, payer, 10, 10000).expect("Should mint.");
    let (quote_mint, _quote_ata) = create_token(rpc, payer, 10, 10000).expect("Should mint.");
    let (deque_pubkey, _deque_bump) = get_deque_address(&base_mint, &quote_mint);
    let (vault_pubkey, _vault_bump) = get_vault_address(&deque_pubkey, &base_mint, &quote_mint);

    println!("deque pubkey {:#?}", deque_pubkey.to_string());
    println!("vault pubkey {:#?}", vault_pubkey.to_string());
    println!("base mint pubkey {:#?}", base_mint.to_string());
    println!("quote mint  pubkey {:#?}", quote_mint.to_string());

    // ------------------------------------- Initialization ----------------------------------------
    println!("Initializing Deque<u64>...");
    let init_data = borsh::to_vec(&DequeInstruction::Initialize {
        deque_type: DequeType::U64.into(),
        num_sectors: 5,
    })
    .expect("Failed to serialize");

    let init_instruction = Instruction::new_with_bytes(
        program_id,
        &init_data,
        vec![
            AccountMeta::new(deque_pubkey, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
    );

    send_txn(
        rpc,
        payer,
        &[payer],
        vec![init_instruction],
        "initialize deque<u64>".to_string(),
    );

    // ---------------------------------------- Mutations ------------------------------------------
    for value in [7u64, 8u64] {
        println!("\nPushing {} to front.", value);
        let push_data = DequeInstruction::PushFront {
            value: value.to_le_bytes().to_vec(),
        };
        send_instruction(
            rpc,
            payer,
            deque_pubkey,
            program_id,
            push_data,
            "push_front",
        );
    }

    // Push some values to the back
    for value in [3u64, 4u64] {
        println!("\nPushing {} to back.", value);
        let push_data = DequeInstruction::PushBack {
            value: value.to_le_bytes().to_vec(),
        };
        send_instruction(rpc, payer, deque_pubkey, program_id, push_data, "push_back");
    }

    // Remove an element
    println!("\nRemoving element at index 1");
    let remove_data = DequeInstruction::Remove { index: 1 };
    send_instruction(rpc, payer, deque_pubkey, program_id, remove_data, "remove");

    // Try to push one more (should have room now)
    println!("\nPushing 777 to back");
    let push_data = DequeInstruction::PushBack {
        value: 777u64.to_le_bytes().to_vec(),
    };
    send_instruction(rpc, payer, deque_pubkey, program_id, push_data, "push_back");

    print_size_and_sectors(rpc, &deque_pubkey);

    // ----------------------------------------- Resize --------------------------------------------
    println!("Resizing Deque<u64>...");
    let additional_sectors = 7;
    let resize_data: Vec<u8> = borsh::to_vec(&DequeInstruction::Resize {
        num_sectors: additional_sectors,
    })
    .expect("Failed to serialize.");

    let resize_ixn = Instruction::new_with_bytes(
        program_id,
        &resize_data,
        vec![
            AccountMeta::new(deque_pubkey, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let mut transaction = Transaction::new_with_payer(&[resize_ixn], Some(&payer.pubkey()));
    let blockhash = rpc.get_latest_blockhash().expect("Failed to get blockhash");
    transaction.sign(&[payer], blockhash);

    match rpc.send_and_confirm_transaction(&transaction) {
        Ok(sig) => println!("✓ Resized: {}", sig),
        Err(e) => {
            eprintln!("Failed to resize: {}", e);
            return;
        }
    }

    // ---------------------------------------- Push more ------------------------------------------
    let start = 71u64;
    let end = start + additional_sectors as u64;
    for i in start..=end {
        send_instruction(
            rpc,
            payer,
            deque_pubkey,
            program_id,
            DequeInstruction::PushFront {
                value: i.to_le_bytes().to_vec(),
            },
            "push front",
        );
    }

    print_size_and_sectors(rpc, &deque_pubkey);
    inspect_account(rpc, &deque_pubkey, false);
}

fn test_market_escrow(rpc: &RpcClient, payer: &Keypair, program_id: Pubkey) {
    // ------------------------------------ Mint two tokens ----------------------------------------
    let (base_mint, _payer_base_ata) = create_token(rpc, payer, 10, 10000).expect("Should mint.");
    let (quote_mint, _payer_quote_ata) = create_token(rpc, payer, 10, 10000).expect("Should mint.");
    let (deque_pubkey, _deque_bump) = get_deque_address(&base_mint, &quote_mint);
    let (vault_pubkey, _vault_bump) = get_vault_address(&deque_pubkey, &base_mint, &quote_mint);

    println!("deque pubkey {:#?}", deque_pubkey.to_string());
    println!("vault pubkey {:#?}", vault_pubkey.to_string());
    println!("base mint pubkey {:#?}", base_mint.to_string());
    println!("quote mint pubkey {:#?}", quote_mint.to_string());

    // ------------------------------------- Initialization ----------------------------------------
    let init_data = borsh::to_vec(&DequeInstruction::Initialize {
        deque_type: DequeType::Market.into(),
        num_sectors: 10,
    })
    .expect("Failed to serialize");

    let (vault_base_ata, vault_quote_ata) = (
        get_associated_token_address(&vault_pubkey, &base_mint),
        get_associated_token_address(&vault_pubkey, &quote_mint),
    );

    let init_instruction = Instruction::new_with_bytes(
        program_id,
        &init_data,
        vec![
            AccountMeta::new(deque_pubkey, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(vault_pubkey, false),
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
    println!("vault_pubkey {:#?}", vault_pubkey.to_string());
    println!("payer_ata {:#?}", payer_ata.to_string());

    // ----------------------------------------- Deposit -------------------------------------------

    println!(
        "Payer balance before: {:?}",
        get_token_balance(rpc, &payer.pubkey(), &base_mint)
    );

    println!(
        "Vault balance before: {:?}",
        get_token_balance(rpc, &vault_pubkey, &base_mint)
    );

    let deposit_ixn = Instruction::new_with_borsh(
        program_id,
        &DequeInstruction::Deposit {
            amount: 1000,
            choice: MarketEscrowChoice::Base,
        },
        vec![
            AccountMeta::new(deque_pubkey, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(vault_pubkey, false),
            AccountMeta::new(payer_ata, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(base_mint, false),
            AccountMeta::new(vault_base_ata, false),
        ],
    );

    send_txn(
        rpc,
        payer,
        &[payer],
        vec![deposit_ixn],
        "deposit".to_string(),
    );

    println!(
        "Payer balance after: {:?}",
        get_token_balance(rpc, &payer.pubkey(), &base_mint)
    );

    println!(
        "Vault balance after: {:?}",
        get_token_balance(rpc, &vault_pubkey, &base_mint)
    );

    inspect_account(rpc, &deque_pubkey, false);

    print_size_and_sectors(rpc, &deque_pubkey);
}

fn print_size_and_sectors(client: &RpcClient, account_pubkey: &Pubkey) {
    if let Ok(account) = client.get_account(account_pubkey) {
        let cloned_data = &mut account.data.clone();
        let deque =
            Deque::new_from_bytes(cloned_data).expect("Should be able to deserialize into Deque.");
        let sector_size = deque.header.get_type().sector_size();
        let len = account.data.len();
        println!(
            "\nAccount size: {} bytes, {} sectors\n",
            len,
            (len - HEADER_FIXED_SIZE) / sector_size
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
                "len: {}, deque_head: {:#?}, deque_tail: {:#?}, free_head: {:#?}, deque_type: {:#?}",
                    deque.header.len,
                    deque.header.deque_head,
                    deque.header.deque_tail,
                    deque.header.free_head,
                    deque.header.deque_type,
            );
            }

            match deque.header.get_type() {
                DequeType::U32 => {
                    let from_head = deque
                        .iter_indices_from_head::<u32>()
                        .map(|it| {
                            *from_sector_idx::<DequeNode<u32>>(deque.sectors, it)
                                .expect("Should be valid.")
                        })
                        .collect::<Vec<DequeNode<u32>>>();
                    println!(
                        "{:?}",
                        from_head.iter().map(|f| f.inner).collect::<Vec<_>>()
                    );
                }
                DequeType::U64 => {
                    let from_head = deque
                        .iter_indices_from_head::<u64>()
                        .map(|it| {
                            *from_sector_idx::<DequeNode<u64>>(deque.sectors, it)
                                .expect("Should be valid.")
                        })
                        .collect::<Vec<DequeNode<u64>>>();
                    println!(
                        "{:?}",
                        from_head.iter().map(|f| f.inner).collect::<Vec<_>>()
                    );
                }
                DequeType::Market => {
                    let from_head = deque
                        .iter_indices_from_head::<u64>()
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
            }
        }
        Err(e) => {
            println!("Failed to get account: {}", e);
        }
    }
}

// Send custom smart contract ixn.
fn send_instruction(
    client: &RpcClient,
    payer: &Keypair,
    deque_account: Pubkey,
    program_id: Pubkey,
    instruction_data: DequeInstruction,
    operation: &str,
) {
    let data = borsh::to_vec(&instruction_data).expect("Failed to serialize");

    let instruction = Instruction::new_with_bytes(
        program_id,
        &data,
        vec![AccountMeta::new(deque_account, false)],
    );

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    let blockhash = client
        .get_latest_blockhash()
        .expect("Failed to get blockhash");
    transaction.sign(&[payer], blockhash);

    match client.send_and_confirm_transaction(&transaction) {
        Ok(sig) => println!("  ✓ {} successful, tx: {}", operation, sig),
        Err(e) => eprintln!("  ✗ {} failed: {}", operation, e),
    }

    inspect_account(client, &deque_account, false);
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
