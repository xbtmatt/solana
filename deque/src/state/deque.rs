use borsh::{BorshDeserialize, BorshSerialize};

// The node index that represents this node's address in the vector of nodes.
type Link = Option<u64>;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Node<T> {
    pub data: T,
    pub prev: Link,
    pub next: Link,
    pub in_use: bool,
}

// Deque, double-ended queue, also known as double linked list.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Deque<T, const N: usize> {
    pub head: Link,
    pub tail: Link,
    // The deque of unused nodes.
    pub free_head: Link,
    // Fixed-size, contiguous bytes representing all nodes- facilitates simple zero copy of data.
    pub nodes: [Node<T>; N],
    pub len: u64,
}

impl<T: Default + BorshSerialize + BorshDeserialize + Clone, const N: usize> Default
    for Deque<T, N>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default + BorshSerialize + BorshDeserialize + Clone, const N: usize> Deque<T, N> {
    pub fn new() -> Self {
        let mut nodes: [Node<T>; N] = std::array::from_fn(|_| Node {
            data: T::default(),
            prev: None,
            next: None,
            in_use: false,
        });

        (0..N).for_each(|i| {
            nodes[i].next = if i + 1 < N {
                Some((i + 1) as u64)
            } else {
                None
            };
            nodes[i].prev = if i > 0 { Some((i - 1) as u64) } else { None };
        });

        Deque {
            head: None,
            tail: None,
            free_head: if N > 0 { Some(0) } else { None },
            nodes,
            len: 0,
        }
    }

    fn alloc_slot(&mut self) -> Option<u64> {
        let idx = self.free_head?;
        // Point the free list to the returned node's next node.
        let next = self.nodes[idx as usize].next;
        self.free_head = next;
        // Then return the index of the newly allocated node.
        Some(idx)
    }

    fn free_slot(&mut self, idx: u64) -> T {
        let node = &mut self.nodes[idx as usize];
        let data = std::mem::take(&mut node.data);
        node.in_use = false;
        node.prev = None;
        node.next = self.free_head;
        self.free_head = Some(idx);
        data
    }

    pub fn push_front(&mut self, value: T) -> Result<u64, &'static str> {
        let idx = self.alloc_slot().ok_or("No more space in list.")?;
        {
            let n = &mut self.nodes[idx as usize];
            n.data = value;
            n.prev = None;
            n.next = self.head;
            n.in_use = true;
        }

        // Some(idx) here essentially acts as the pointer to the node.
        match self.head {
            None => self.tail = Some(idx),
            Some(head_idx) => self.nodes[head_idx as usize].prev = Some(idx),
        }

        self.head = Some(idx);
        self.len += 1;
        Ok(idx)
    }

    pub fn push_back(&mut self, value: T) -> Result<u64, &'static str> {
        let idx = self.alloc_slot().ok_or("No more space in list.")?;
        {
            let n = &mut self.nodes[idx as usize];
            n.data = value;
            n.prev = self.tail;
            n.next = None;
            n.in_use = true;
        }

        // Some(idx) here essentially acts as the pointer to the node.
        match self.tail {
            None => self.head = Some(idx),
            Some(tail_idx) => self.nodes[tail_idx as usize].next = Some(idx),
        }

        self.tail = Some(idx);
        self.len += 1;
        Ok(idx)
    }

    pub fn remove(&mut self, i: u64) -> Result<T, &'static str> {
        let idx = i as usize;
        if idx >= N || !self.nodes[idx].in_use {
            return Err("Invalid index passed.");
        }

        let (prev, next) = {
            let node = &self.nodes[idx];
            (node.prev, node.next)
        };

        match prev {
            Some(prev_idx) => self.nodes[prev_idx as usize].next = next,
            // The removed node is the head node.
            None => self.head = next,
        }

        match next {
            Some(next_idx) => self.nodes[next_idx as usize].prev = prev,
            None => self.tail = prev,
        }

        // Free up the slot and return the extracted data.
        let data = self.free_slot(i);
        self.len -= 1;

        Ok(data)
    }

    pub fn iter_indices_from_head(&self) -> impl Iterator<Item = u64> + '_ {
        std::iter::successors(self.head, move |&idx| self.nodes[idx as usize].next)
    }

    pub fn iter_indices_from_tail(&self) -> impl Iterator<Item = u64> + '_ {
        std::iter::successors(self.tail, move |&idx| self.nodes[idx as usize].prev)
    }
}
