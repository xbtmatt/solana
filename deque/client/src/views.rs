use deque::{
    state::{Deque, DequeNode, MarketEscrow},
    utils::from_sector_idx,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

pub fn inspect_account(client: &RpcClient, account_pubkey: &Pubkey, verbose: bool) {
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
