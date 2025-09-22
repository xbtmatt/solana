use deque::events::EmittableEvent;
use solana_sdk::signature::Signature;

pub fn parse_events<T: EmittableEvent>(sig: &Signature, events: Vec<Vec<u8>>) -> Vec<T> {
    vec![]
}
