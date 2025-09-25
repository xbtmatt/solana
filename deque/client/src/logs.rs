use deque::{state::DEQUE_HEADER_SIZE, utils::SECTOR_SIZE};
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

pub fn print_size_and_sectors(client: &RpcClient, account_pubkey: &Pubkey) {
    if let Ok(account) = client.get_account(account_pubkey) {
        let len = account.data.len();
        println!(
            "\nAccount size: {} bytes, {} sectors\n",
            len,
            (len - DEQUE_HEADER_SIZE) / SECTOR_SIZE
        );
    }
}

pub fn bytes_to_str(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
