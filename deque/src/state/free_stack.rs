use bytemuck::{Pod, Zeroable};
use solana_program::program_error::ProgramError;

use crate::{
    state::DequeNode,
    utils::{from_slot_mut, Slab, SlotIndex, NIL},
};

/// NIL/LAST are interchangeable within the context of the stack structure.
const LAST: u32 = NIL;
pub struct Stack<'a, T: Pod> {
    pub head: SlotIndex,
    pub data: &'a mut [u8],
    pub phantom: std::marker::PhantomData<&'a T>,
}

#[derive(Clone, Copy, Zeroable)]
#[repr(C)]
pub struct StackNode<T> {
    /// The zeroed inner payload bytes.
    pub inner: T,
    pub next: SlotIndex,
    /// Add a dummy field to align perfectly with the deque node.
    pub _dummy: SlotIndex,
}

unsafe impl<T: Pod> Pod for StackNode<T> {}

impl<T: Pod> Slab for StackNode<T> {}

impl<'a, T: Pod> Stack<'a, T> {
    /// Initialize from a byte vector; it's expected that it's already well-formed.
    pub fn new(data: &'a mut [u8], head: SlotIndex) -> Self {
        debug_assert_eq!(size_of::<StackNode<T>>(), size_of::<DequeNode<T>>());

        Stack {
            head,
            data,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn push_to_free(&mut self, idx: SlotIndex) -> Result<(), ProgramError> {
        let node: &mut StackNode<T> = from_slot_mut::<StackNode<T>>(self.data, idx)?;
        node.inner = T::zeroed();
        node.next = self.head;
        self.head = idx;

        Ok(())
    }

    pub fn remove_from_free(&mut self) -> Result<SlotIndex, ProgramError> {
        if self.head == LAST {
            return Ok(LAST);
        }

        let removed_idx = self.head;
        let head = from_slot_mut::<StackNode<T>>(self.data, removed_idx)?;
        self.head = head.next;

        // Fully zero out the node by setting `next` to 0.
        head.next = 0;

        Ok(removed_idx)
    }

    pub fn get_head(&self) -> SlotIndex {
        self.head
    }
}
