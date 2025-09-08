use bytemuck::{Pod, Zeroable};
use solana_program::program_error::ProgramError;

use crate::utils::{from_slab_bytes_mut, Index, SlabElement, NIL};

/// NIL/LAST are interchangeable within the context of the stack structure.
const LAST: u32 = NIL;
pub struct FreeNodeStack<'a, T: Pod> {
    head: Index,
    data: &'a mut [u8],
    phantom: std::marker::PhantomData<&'a T>,
}

#[derive(Clone, Copy, Zeroable)]
#[repr(C)]
pub struct FreeNode<T> {
    next: Index,
    /// The zeroed inner payload bytes.
    inner: T,
}

unsafe impl<T: Pod> Pod for FreeNode<T> {}

impl<T: Pod> SlabElement for FreeNode<T> {}

impl<'a, T: Pod> FreeNodeStack<'a, T> {
    /// Initialize a new stack from a byte vector that is already a well-formed byte
    /// representation of one.
    pub fn new(data: &'a mut [u8], head: Index) -> Self {
        FreeNodeStack {
            head,
            data,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn push_to_free(&mut self, idx: Index) -> Result<(), ProgramError> {
        let node: &mut FreeNode<T> = from_slab_bytes_mut::<FreeNode<T>>(self.data, idx)?;
        node.inner = T::zeroed();
        node.next = self.head;
        self.head = idx;

        Ok(())
    }

    pub fn remove_from_free(&mut self) -> Result<Index, ProgramError> {
        if self.head == LAST {
            return Ok(LAST);
        }

        let removed_idx = self.head;
        let head = from_slab_bytes_mut::<FreeNode<T>>(self.data, removed_idx)?;
        self.head = head.next;

        // Fully zero out the node by setting `next` to 0.
        head.next = 0;

        Ok(removed_idx)
    }
}
