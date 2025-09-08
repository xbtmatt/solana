use borsh::{BorshDeserialize, BorshSerialize};
use deque::{
    state::{Deque, DequeAccount, DequeInstruction, Link, NIL},
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
use std::str::FromStr;

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

    println!("\n=== Testing Deque<u32, 10> ===");
    let deque_u32 = Keypair::new();
    test_u32_deque(&client, &payer, &deque_u32, program_id);
}

fn test_u64_deque(
    client: &RpcClient,
    payer: &Keypair,
    deque_account: &Keypair,
    program_id: Pubkey,
) {
    // Initialize deque for u64s (type 0)
    println!("Initializing Deque<u64, 5>...");
    let init_data = borsh::to_vec(&DequeInstruction::Initialize { deque_type: 0 })
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

    // Push some values to the front
    println!("\nPushing values to front: 100, 200");
    for value in [100u64, 200u64] {
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
    println!("\nPushing values to back: 300, 400");
    for value in [300u64, 400u64] {
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
    println!("\nPushing 500 to back");
    let push_data = DequeInstruction::PushBack {
        value: 500u64.to_le_bytes().to_vec(),
    };
    send_instruction(
        client,
        payer,
        deque_account.pubkey(),
        program_id,
        push_data,
        "push_back",
    );

    // Read and display the account data
    println!("\nFinal deque state:");
    if let Ok(account) = client.get_account(&deque_account.pubkey()) {
        println!("Account size: {} bytes", account.data.len());
        // In a real scenario, you'd deserialize and iterate through the deque
        // For now, just show that the account exists and has data
    }

    println!("\n=== Inspecting initialized Five u64s account ===");
    inspect_account(client, &deque_account.pubkey());
}

fn test_u32_deque(
    client: &RpcClient,
    payer: &Keypair,
    deque_account: &Keypair,
    program_id: Pubkey,
) {
    // Initialize deque for u32s (type 1)
    println!("Initializing Deque<u32, 10>...");
    let init_data = borsh::to_vec(&DequeInstruction::Initialize { deque_type: 1 })
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

    // Push values alternating front and back
    println!("\nPushing values alternating front/back");
    let values: Vec<(u32, bool)> = vec![
        (10, true),  // front
        (20, false), // back
        (30, true),  // front
        (40, false), // back
        (50, true),  // front
        (60, false), // back
        (70, true),  // front
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
    println!("\nPushing 80 and 90 to back");
    for value in [80u32, 90u32] {
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

    // Read and display the account data
    println!("\nFinal deque state:");
    if let Ok(account) = client.get_account(&deque_account.pubkey()) {
        println!("Account size: {} bytes", account.data.len());
        // Expected order after all operations (conceptually):
        // Front: 70, 50, 30, (removed), 10, (removed), 20, 40, 60, 80, 90 :Back
    }

    println!("\n=== Inspecting initialized Ten u32s account ===");
    inspect_account(client, &deque_account.pubkey());
}
fn inspect_account(client: &RpcClient, account_pubkey: &Pubkey) {
    match client.get_account(account_pubkey) {
        Ok(account) => {
            println!("Account owner: {}", account.owner);
            println!("Account lamports: {}", account.lamports);
            println!("Account data length: {} bytes", account.data.len());
            println!("Account executable: {}", account.executable);

            // Display raw bytes (first 100 or so)
            println!("\nRaw data (hex):");
            let display_len = std::cmp::min(account.data.len(), 100);
            for (i, chunk) in account.data[..display_len].chunks(16).enumerate() {
                print!("{:04x}: ", i * 16);
                for byte in chunk {
                    print!("{:02x} ", byte);
                }
                println!();
            }

            // Try to deserialize as DequeAccount
            println!("\nAttempting to deserialize...");
            match DequeAccount::try_from_slice(&account.data) {
                Ok(deque) => match deque {
                    DequeAccount::FiveU64s(d) => {
                        println!("Order(head→tail): {:?}", collect_from_head_u64(&d));
                    }
                    DequeAccount::TenU32s(d) => {
                        println!("Order(head→tail): {:?}", collect_from_head_u32(&d));
                    }
                },
                Err(e) => {
                    println!("Failed to deserialize: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Failed to get account: {}", e);
        }
    }
}

fn collect_from_head_u64<const N: usize>(d: &Deque<u64, N>) -> Vec<u64> {
    let mut out = Vec::new();
    let mut cur: Link = d.head;
    while cur != NIL {
        let u = cur as usize;
        if d.nodes[u].in_use {
            out.push(d.nodes[u].data);
        }
        cur = d.nodes[u].next;
    }
    out
}

fn collect_from_head_u32<const N: usize>(d: &Deque<u32, N>) -> Vec<u32> {
    let mut out = Vec::new();
    let mut cur: Link = d.head;
    while cur != NIL {
        let u = cur as usize;
        if d.nodes[u].in_use {
            out.push(d.nodes[u].data);
        }
        cur = d.nodes[u].next;
    }
    out
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
        Ok(_sig) => println!("  ✓ {} successful", operation),
        Err(e) => eprintln!("  ✗ {} failed: {}", operation, e),
    }
}
