// The node index that represents this node's address in the vector of nodes.
type Link = Option<u64>;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Node<T> {
    pub data: T,
    pub prev: Link,
    pub next: Link,
    pub in_use: bool;
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct DoubleLinkedList<T, const N: usize> {
    pub head: Link;
    pub tail: Link;
    // The double linked list of unused nodes.
    pub free_head: Link;
    // Fixed-size, contiguous bytes representing all nodes- facilitates simple zero copy of data.
    pub nodes: [Node<T>; N],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path() {

    }
}
