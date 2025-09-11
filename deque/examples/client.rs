use deque::{
    state::{
        get_deque_address, get_vault_address, Deque, DequeInstruction, DequeNode, DequeType,
        HEADER_FIXED_SIZE,
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

    // Test Deque with 5 u64s
    println!("=== Testing Deque<u64, 5> ===");
    test_u64_deque(&client, &payer, program_id);
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

    let token_acc = Keypair::new();
    let acc_space = spl_token::state::Account::LEN;
    let acc_rent = rpc
        .get_minimum_balance_for_rent_exemption(acc_space)
        .or(Err(()))?;
    let create_acc = system_instruction::create_account(
        &payer.pubkey(),
        &token_acc.pubkey(),
        acc_rent,
        acc_space as u64,
        &spl_token::id(),
    );

    let init_acc = spl_token::instruction::initialize_account3(
        &spl_token::id(),
        &token_acc.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .or(Err(()))?;

    let mint_to = spl_token::instruction::mint_to_checked(
        &spl_token::id(),
        &mint.pubkey(),
        &token_acc.pubkey(),
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
        "mint one".to_string(),
    );
    send_txn(
        rpc,
        payer,
        &[&token_acc],
        vec![create_acc, init_acc, mint_to],
        "mint two".to_string(),
    );

    Ok((mint.pubkey(), token_acc.pubkey()))
}

fn test_u64_deque(rpc: &RpcClient, payer: &Keypair, program_id: Pubkey) {
    // ------------------------------------ Mint two tokens ----------------------------------------
    let (base_mint, _base_token_acc) = create_token(rpc, payer, 10, 10000).expect("Should mint.");
    let (quote_mint, _quote_token_acc) = create_token(rpc, payer, 10, 10000).expect("Should mint.");
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
        base_mint,
        quote_mint,
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

    let mut transaction = Transaction::new_with_payer(&[init_instruction], Some(&payer.pubkey()));
    let blockhash = rpc.get_latest_blockhash().expect("Failed to get blockhash");
    transaction.sign(&[payer], blockhash);

    match rpc.send_and_confirm_transaction(&transaction) {
        Ok(sig) => println!("✓ Initialized: {}", sig),
        Err(e) => {
            eprintln!("Failed to initialize: {}", e);
            return;
        }
    }

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
                    // let free_head = from_sector::<DequeNode<u32>>(sectors, header.free_head);
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
                    // let free_head = from_sector::<DequeNode<u64>>(sectors, header.free_head);
                    println!(
                        "{:?}",
                        from_head.iter().map(|f| f.inner).collect::<Vec<_>>()
                    );
                }
                DequeType::Market => {
                    todo!();
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
    ix: Vec<Instruction>,
    txn_label: String,
) {
    let bh = rpc
        .get_latest_blockhash()
        .or(Err(()))
        .expect("Should be able to get blockhash.");
    let msg = Message::new(&ix, Some(&payer.pubkey()));
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
