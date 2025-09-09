use bytemuck::{Pod, Zeroable};
use solana_program::program_error::ProgramError;

use crate::{
    state::{DequeHeader, Stack, StackNode},
    utils::{from_slab_bytes, from_slab_bytes_mut, Slab, SlotIndex, NIL},
};

#[derive(Clone, Copy, Zeroable)]
#[repr(C)]
pub struct DequeNode<T> {
    // The inner payload bytes.
    inner: T,
    prev: SlotIndex,
    next: SlotIndex,
}

unsafe impl<T: Pod> Pod for DequeNode<T> {}

impl<T: Pod> Slab for DequeNode<T> {}

pub struct DequeAccount<'a> {
    pub header: &'a mut DequeHeader,
    // Either StackNode<T> or DequeNode<T>
    pub slots: &'a mut [u8],
}

pub type DequeValue = (DequeHeader, Vec<u8>);

impl<'a> DequeAccount<'a> {
    pub fn new_from_bytes(data: &'a mut [u8]) -> Result<Self, ProgramError> {
        let (header_slab, slots) = data.split_at_mut(size_of::<DequeHeader>());
        let header = from_slab_bytes_mut::<DequeHeader>(header_slab, 0_u32)?;

        Ok(Self { header, slots })
    }

    pub fn as_free_mut<P: Pod>(
        &mut self,
        idx: SlotIndex,
    ) -> Result<&mut StackNode<P>, ProgramError> {
        let free_node = from_slab_bytes_mut::<StackNode<P>>(self.slots, idx)?;
        Ok(free_node)
    }

    pub fn as_deque_mut<P: Pod>(
        &mut self,
        idx: SlotIndex,
    ) -> Result<&mut DequeNode<P>, ProgramError> {
        let deque_node = from_slab_bytes_mut::<DequeNode<P>>(self.slots, idx)?;
        Ok(deque_node)
    }

    pub fn push_front<P: Pod + Zeroable>(&mut self, value: P) -> Result<(), ProgramError> {
        let mut free = Stack::<P>::new(self.slots, self.header.free_head);
        let free_idx = free.remove_from_free()?;
        self.header.free_head = free.get_head();
        if free_idx == NIL {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let head = self.header.deque_head;
        let n: &mut DequeNode<P> = from_slab_bytes_mut(self.slots, free_idx)?;
        *n = DequeNode {
            inner: value,
            prev: NIL,
            next: head,
        };

        match head {
            NIL => self.header.deque_tail = free_idx,
            head => from_slab_bytes_mut::<DequeNode<P>>(self.slots, head)?.prev = free_idx,
        }

        self.header.deque_head = free_idx;
        self.header.len = self.header.len.saturating_add(1);
        Ok(())
    }

    pub fn push_back<P: Pod + Zeroable>(&mut self, value: P) -> Result<(), ProgramError> {
        let mut free = Stack::<P>::new(self.slots, self.header.free_head);
        let free_idx = free.remove_from_free()?;
        self.header.free_head = free.get_head();
        if free_idx == NIL {
            return Err(ProgramError::InvalidAccountData);
        }

        let tail = self.header.deque_tail;
        let n: &mut DequeNode<P> = from_slab_bytes_mut(self.slots, free_idx)?;
        *n = DequeNode {
            inner: value,
            prev: tail,
            next: NIL,
        };

        match tail {
            NIL => self.header.deque_head = free_idx,
            tail => from_slab_bytes_mut::<DequeNode<P>>(self.slots, tail)?.next = free_idx,
        }

        self.header.deque_tail = free_idx;
        self.header.len = self.header.len.saturating_add(1);
        Ok(())
    }

    pub fn remove<P: Pod + Zeroable>(&mut self, i: SlotIndex) -> Result<P, ProgramError> {
        if i == NIL {
            return Err(ProgramError::InvalidInstructionData);
        };

        let (prev, next, inner) = {
            let n: &DequeNode<P> = from_slab_bytes::<DequeNode<P>>(self.slots, i)?;
            (n.prev, n.next, n.inner)
        };

        match prev {
            NIL => self.header.deque_head = next,
            prev => from_slab_bytes_mut::<DequeNode<P>>(self.slots, prev)?.next = next,
        }

        match next {
            NIL => self.header.deque_tail = prev,
            next => from_slab_bytes_mut::<DequeNode<P>>(self.slots, next)?.prev = prev,
        }

        self.header.len = self.header.len.saturating_sub(1);
        let mut free = Stack::<P>::new(self.slots, self.header.free_head);
        free.push_to_free(i)?;
        self.header.free_head = free.get_head();
        Ok(inner)
    }

    pub fn iter_indices_from_head<T: Slab>(&self) -> impl Iterator<Item = SlotIndex> + '_ {
        let start = (self.header.deque_head != NIL).then_some(self.header.deque_head);
        std::iter::successors(start, move |&i| {
            let node = from_slab_bytes::<DequeNode<T>>(self.slots, i).ok()?;
            (node.next != NIL).then_some(node.next)
        })
    }

    pub fn iter_indices_from_tail<T: Slab>(&self) -> impl Iterator<Item = SlotIndex> + '_ {
        let start = (self.header.deque_tail != NIL).then_some(self.header.deque_tail);
        std::iter::successors(start, move |&i| {
            let node = from_slab_bytes::<DequeNode<T>>(self.slots, i).ok()?;
            (node.prev != NIL).then_some(node.prev)
        })
    }
}
