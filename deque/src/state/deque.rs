use borsh::{BorshDeserialize, BorshSerialize};

// The node index that represents this node's address in the vector of nodes.
// NIL as u16::MAX is used to ensure a fixed size for serialization.
pub type Link = u16;
pub const NIL: Link = Link::MAX;

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
        let nodes: [Node<T>; N] = std::array::from_fn(|i| Node {
            data: T::default(),
            prev: if i > 0 { (i as Link) - 1 } else { NIL },
            next: if i + 1 < N { (i as Link) + 1 } else { NIL },
            in_use: false,
        });

        Deque {
            head: NIL,
            tail: NIL,
            free_head: if N > 0 { 0 } else { NIL },
            nodes,
            len: 0,
        }
    }

    #[inline]
    fn alloc_slot(&mut self) -> Option<Link> {
        let idx = self.free_head;
        if idx == NIL {
            return None;
        }
        // Point the free list to the returned node's next node.
        let next = self.nodes[idx as usize].next;
        self.free_head = next;

        let n = &mut self.nodes[idx as usize];
        n.prev = NIL;
        n.next = NIL;
        n.in_use = true;
        // Then return the index of the newly allocated node.
        Some(idx)
    }

    #[inline]
    fn free_slot(&mut self, idx: Link) -> T {
        let node = &mut self.nodes[idx as usize];
        let data = std::mem::take(&mut node.data);
        node.in_use = false;
        node.prev = NIL;
        node.next = self.free_head;
        self.free_head = idx;
        data
    }

    pub fn push_front(&mut self, value: T) -> Result<Link, &'static str> {
        let idx = self.alloc_slot().ok_or("No more space in list.")?;
        {
            let n = &mut self.nodes[idx as usize];
            n.data = value;
            n.prev = NIL;
            n.next = self.head;
            n.in_use = true;
        }

        match self.head {
            NIL => self.tail = idx,
            head_idx => self.nodes[head_idx as usize].prev = idx,
        }

        self.head = idx;
        self.len = self.len.saturating_add(1);
        Ok(idx)
    }

    pub fn push_back(&mut self, value: T) -> Result<Link, &'static str> {
        let idx = self.alloc_slot().ok_or("No more space in list.")?;
        {
            let n = &mut self.nodes[idx as usize];
            n.data = value;
            n.prev = self.tail;
            n.next = NIL;
            n.in_use = true;
        }

        match self.tail {
            NIL => self.head = idx,
            tail_idx => self.nodes[tail_idx as usize].next = idx,
        }

        self.tail = idx;
        self.len = self.len.saturating_add(1);
        Ok(idx)
    }

    pub fn remove(&mut self, i: Link) -> Result<T, &'static str> {
        let idx = i as usize;
        if i == NIL || idx >= N || !self.nodes[idx].in_use {
            return Err("Invalid index passed.");
        }

        let (prev, next) = {
            let node = &self.nodes[idx];
            (node.prev, node.next)
        };

        match prev {
            // The removed node is the head node.
            NIL => self.head = next,
            prev_idx => self.nodes[prev_idx as usize].next = next,
        }

        match next {
            NIL => self.tail = prev,
            next_idx => self.nodes[next_idx as usize].prev = prev,
        }

        // Free up the slot and return the extracted data.
        let data = self.free_slot(i);
        self.len = self.len.saturating_sub(1);

        Ok(data)
    }

    pub fn iter_indices_from_head(&self) -> impl Iterator<Item = Link> + '_ {
        std::iter::successors((self.head != NIL).then_some(self.head), move |&i| {
            let j = self.nodes[i as usize].next;
            (j != NIL).then_some(j)
        })
    }

    pub fn iter_indices_from_tail(&self) -> impl Iterator<Item = Link> + '_ {
        std::iter::successors((self.tail != NIL).then_some(self.tail), move |&i| {
            let j = self.nodes[i as usize].prev;
            (j != NIL).then_some(j)
        })
    }
}
