use deque::{
    state::{Deque, DequeInstruction, DequeNode, DequeType, HEADER_FIXED_SIZE},
    utils::from_sector_idx,
    PROGRAM_ID_PUBKEY,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

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
    let deque_u64 = Keypair::new();
    println!("{:#?}", deque_u64.pubkey().to_string());
    test_u64_deque(&client, &payer, &deque_u64, program_id);
    inspect_account(&client, &deque_u64.pubkey(), false);

    println!("\n=== Testing Deque<u32, 10> ===");
    let deque_u32 = Keypair::new();
    test_u32_deque(&client, &payer, &deque_u32, program_id);
    inspect_account(&client, &deque_u32.pubkey(), false);
}

fn test_u64_deque(
    client: &RpcClient,
    payer: &Keypair,
    deque_account: &Keypair,
    program_id: Pubkey,
) {
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
            AccountMeta::new(deque_account.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let mut transaction = Transaction::new_with_payer(&[init_instruction], Some(&payer.pubkey()));
    let blockhash = client
        .get_latest_blockhash()
        .expect("Failed to get blockhash");
    transaction.sign(&[payer, deque_account], blockhash);

    match client.send_and_confirm_transaction(&transaction) {
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
            client,
            payer,
            deque_account.pubkey(),
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
        send_instruction(
            client,
            payer,
            deque_account.pubkey(),
            program_id,
            push_data,
            "push_back",
        );
    }

    // Remove an element
    println!("\nRemoving element at index 1");
    let remove_data = DequeInstruction::Remove { index: 1 };
    send_instruction(
        client,
        payer,
        deque_account.pubkey(),
        program_id,
        remove_data,
        "remove",
    );

    // Try to push one more (should have room now)
    println!("\nPushing 777 to back");
    let push_data = DequeInstruction::PushBack {
        value: 777u64.to_le_bytes().to_vec(),
    };
    send_instruction(
        client,
        payer,
        deque_account.pubkey(),
        program_id,
        push_data,
        "push_back",
    );

    print_size_and_sectors(client, deque_account);

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
            AccountMeta::new(deque_account.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let mut transaction = Transaction::new_with_payer(&[resize_ixn], Some(&payer.pubkey()));
    let blockhash = client
        .get_latest_blockhash()
        .expect("Failed to get blockhash");
    transaction.sign(&[payer, deque_account], blockhash);

    match client.send_and_confirm_transaction(&transaction) {
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
            client,
            payer,
            deque_account.pubkey(),
            program_id,
            DequeInstruction::PushFront {
                value: i.to_le_bytes().to_vec(),
            },
            "push front",
        );
    }

    print_size_and_sectors(client, deque_account);
}

fn test_u32_deque(
    client: &RpcClient,
    payer: &Keypair,
    deque_account: &Keypair,
    program_id: Pubkey,
) {
    // ------------------------------------- Initialization ----------------------------------------
    println!("Initializing Deque<u32>...");
    let init_data = borsh::to_vec(&DequeInstruction::Initialize {
        deque_type: DequeType::U32.into(),
        num_sectors: 10,
    })
    .expect("Failed to serialize");

    let init_instruction = Instruction::new_with_bytes(
        program_id,
        &init_data,
        vec![
            AccountMeta::new(deque_account.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let mut transaction = Transaction::new_with_payer(&[init_instruction], Some(&payer.pubkey()));
    let blockhash = client
        .get_latest_blockhash()
        .expect("Failed to get blockhash");
    transaction.sign(&[payer, deque_account], blockhash);

    match client.send_and_confirm_transaction(&transaction) {
        Ok(sig) => println!("✓ Initialized: {}", sig),
        Err(e) => {
            eprintln!("Failed to initialize: {}", e);
            return;
        }
    }

    // ---------------------------------------- Mutations ------------------------------------------
    // Push values alternating front and back
    println!("\nPushing values alternating front/back");
    let values: Vec<(u32, bool)> = vec![
        (5, true),   // front
        (6, false),  // back
        (7, true),   // front
        (8, false),  // back
        (9, true),   // front
        (10, false), // back
        (11, true),  // front
    ];

    for (value, is_front) in values {
        let push_data = if is_front {
            println!("  Push {} to front", value);
            DequeInstruction::PushFront {
                value: value.to_le_bytes().to_vec(),
            }
        } else {
            println!("  Push {} to back", value);
            DequeInstruction::PushBack {
                value: value.to_le_bytes().to_vec(),
            }
        };
        let op = if is_front { "push_front" } else { "push_back" };
        send_instruction(
            client,
            payer,
            deque_account.pubkey(),
            program_id,
            push_data,
            op,
        );
    }

    // Remove a couple elements
    println!("\nRemoving elements at indices 2 and 4");
    for index in [2, 4] {
        let remove_data = DequeInstruction::Remove { index };
        send_instruction(
            client,
            payer,
            deque_account.pubkey(),
            program_id,
            remove_data,
            "remove",
        );
    }

    // Add a couple more
    println!("\nPushing 10 and 11 to back");
    for value in [10u32, 11u32] {
        let push_data = DequeInstruction::PushBack {
            value: value.to_le_bytes().to_vec(),
        };
        send_instruction(
            client,
            payer,
            deque_account.pubkey(),
            program_id,
            push_data,
            "push_back",
        );
    }

    print_size_and_sectors(client, deque_account);

    // ----------------------------------------- Resize --------------------------------------------
    println!("Resizing Deque<u32>...");
    let additional_sectors = 3;
    let resize_data: Vec<u8> = borsh::to_vec(&DequeInstruction::Resize {
        num_sectors: additional_sectors,
    })
    .expect("Failed to serialize.");

    let resize_ixn = Instruction::new_with_bytes(
        program_id,
        &resize_data,
        vec![
            AccountMeta::new(deque_account.pubkey(), true),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    let mut transaction = Transaction::new_with_payer(&[resize_ixn], Some(&payer.pubkey()));
    let blockhash = client
        .get_latest_blockhash()
        .expect("Failed to get blockhash");
    transaction.sign(&[payer, deque_account], blockhash);

    match client.send_and_confirm_transaction(&transaction) {
        Ok(sig) => println!("✓ Resized: {}", sig),
        Err(e) => {
            eprintln!("Failed to resize: {}", e);
            return;
        }
    }

    print_size_and_sectors(client, deque_account);

    // ---------------------------------------- Push more ------------------------------------------
    let start = 31u32;
    let end = start + additional_sectors as u32;
    for i in start..=end {
        send_instruction(
            client,
            payer,
            deque_account.pubkey(),
            program_id,
            DequeInstruction::PushFront {
                value: i.to_le_bytes().to_vec(),
            },
            "push front",
        );
    }

    // --------------------------------------- Print size ------------------------------------------
}

fn print_size_and_sectors(client: &RpcClient, account: &Keypair) {
    if let Ok(account) = client.get_account(&account.pubkey()) {
        let cloned_data = &mut account.data.clone();
        let deque =
            Deque::new_from_bytes(cloned_data).expect("Should be able to deserialize into Deque.");
        let slot_size = deque.header.get_type().slot_size();
        let len = account.data.len();
        println!(
            "\nAccount size: {} bytes, {} sectors\n",
            len,
            (len - HEADER_FIXED_SIZE) / slot_size
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
                    // let free_head = from_slot::<DequeNode<u32>>(sectors, header.free_head);
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
                    // let free_head = from_slot::<DequeNode<u64>>(sectors, header.free_head);
                    println!(
                        "{:?}",
                        from_head.iter().map(|f| f.inner).collect::<Vec<_>>()
                    );
                }
            }

            // println!("{:#?}", header);
            // println!("{:#?}", sectors);

            // Try to deserialize as DequeAccount.
            // match Deque::as_deque_mut(&account.data) {
            // }
        }
        Err(e) => {
            println!("Failed to get account: {}", e);
        }
    }
}

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
