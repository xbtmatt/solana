use bytemuck::{Pod, Zeroable};
use solana_program::{msg, program_error::ProgramError};

use crate::{
    state::{DequeHeader, DequeType, Stack, StackNode, HEADER_FIXED_SIZE},
    utils::{from_slab_bytes_mut, from_slot, from_slot_mut, Slab, SlotIndex, NIL},
};

#[derive(Clone, Copy, Debug, Zeroable)]
#[repr(C)]
pub struct DequeNode<T> {
    // The inner payload bytes.
    pub inner: T,
    pub prev: SlotIndex,
    pub next: SlotIndex,
}

unsafe impl<T: Pod> Pod for DequeNode<T> {}

impl<T: Pod> Slab for DequeNode<T> {}

pub struct Deque<'a> {
    pub header: &'a mut DequeHeader,
    // Either StackNode<T> or DequeNode<T>
    pub slots: &'a mut [u8],
}

pub type DequeValue = (DequeHeader, Vec<u8>);

impl<'a> Deque<'a> {
    /// Construct a new, empty Deque, given an existing header and the deque type input.
    /// It's assumed that the data has already been allocated and aligned properly to the number
    /// of slots.
    pub fn init_deque_account(
        data: &mut [u8],
        deque_type: DequeType,
        num_slots: u16,
    ) -> Result<(), ProgramError> {
        if data.len() < HEADER_FIXED_SIZE {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut deque = Deque::new_from_bytes(data)?;
        // Write a new empty header to the `deque.header`
        *deque.header = DequeHeader::new_empty(deque_type);

        let slot_size = deque_type.slot_size();
        debug_assert_eq!(deque.slots.len() % slot_size, 0);
        debug_assert_eq!(deque.slots.len(), (num_slots as usize) * slot_size);

        let space_needed = (num_slots as usize)
            .checked_mul(slot_size)
            .ok_or(ProgramError::InvalidAccountData)?;
        if deque.slots.len() < space_needed {
            return Err(ProgramError::InvalidAccountData);
        }

        match deque_type {
            DequeType::U32 => deque.init_free_stack::<u32>(num_slots as usize)?,
            DequeType::U64 => deque.init_free_stack::<u64>(num_slots as usize)?,
        }

        Ok(())
    }

    pub fn init_free_stack<T: Pod + Zeroable>(
        &mut self,
        num_slots: usize,
    ) -> Result<(), ProgramError> {
        let mut stack = Stack::<T>::new(self.slots, self.header.free_head);
        for s in (0..num_slots).rev() {
            stack.push_to_free(s as SlotIndex)?;
        }
        self.header.free_head = stack.get_head();
        Ok(())
    }

    /// Construct a Deque from an existing byte vector- assumed to be well-formed.
    pub fn new_from_bytes(data: &'a mut [u8]) -> Result<Self, ProgramError> {
        let (header_slab, slots) = data.split_at_mut(HEADER_FIXED_SIZE);
        let header = from_slab_bytes_mut::<DequeHeader>(header_slab, 0_usize)?;
        Ok(Self { header, slots })
    }

    pub fn as_free_mut<P: Pod>(
        &mut self,
        idx: SlotIndex,
    ) -> Result<&mut StackNode<P>, ProgramError> {
        let free_node = from_slot_mut::<StackNode<P>>(self.slots, idx)?;
        Ok(free_node)
    }

    pub fn as_deque_mut<P: Pod>(
        &mut self,
        idx: SlotIndex,
    ) -> Result<&mut DequeNode<P>, ProgramError> {
        let deque_node = from_slot_mut::<DequeNode<P>>(self.slots, idx)?;
        Ok(deque_node)
    }

    pub fn push_front<P: Pod + Zeroable + std::fmt::Debug>(
        &mut self,
        value: P,
    ) -> Result<SlotIndex, ProgramError> {
        msg!("pushing {:#?} to front", value);
        let mut free = Stack::<P>::new(self.slots, self.header.free_head);
        let new_idx = free.remove_from_free()?;
        self.header.free_head = free.get_head();
        if new_idx == NIL {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let head = self.header.deque_head;
        let n: &mut DequeNode<P> = from_slot_mut(self.slots, new_idx)?;
        *n = DequeNode {
            inner: value,
            prev: NIL,
            next: head,
        };

        match head {
            NIL => self.header.deque_tail = new_idx,
            head => from_slot_mut::<DequeNode<P>>(self.slots, head)?.prev = new_idx,
        }

        self.header.deque_head = new_idx;
        self.header.len = self.header.len.saturating_add(1);
        Ok(new_idx)
    }

    pub fn push_back<P: Pod + Zeroable>(&mut self, value: P) -> Result<SlotIndex, ProgramError> {
        let mut free = Stack::<P>::new(self.slots, self.header.free_head);
        let new_idx = free.remove_from_free()?;
        self.header.free_head = free.get_head();
        if new_idx == NIL {
            return Err(ProgramError::InvalidAccountData);
        }

        let tail = self.header.deque_tail;
        let n: &mut DequeNode<P> = from_slot_mut(self.slots, new_idx)?;
        *n = DequeNode {
            inner: value,
            prev: tail,
            next: NIL,
        };

        match tail {
            NIL => self.header.deque_head = new_idx,
            tail => from_slot_mut::<DequeNode<P>>(self.slots, tail)?.next = new_idx,
        }

        self.header.deque_tail = new_idx;
        self.header.len = self.header.len.saturating_add(1);
        Ok(new_idx)
    }

    pub fn remove<P: Pod + Zeroable + std::fmt::Debug>(
        &mut self,
        pos: SlotIndex,
    ) -> Result<P, ProgramError> {
        let len = self.header.len;
        if pos >= len {
            return Err(ProgramError::InvalidArgument);
        }

        // Pick the closer direction, grab the slot index
        let idx = if pos <= len / 2 {
            self.iter_indices_from_head::<P>().nth(pos as usize)
        } else {
            self.iter_indices_from_tail::<P>()
                .nth((len - 1 - pos) as usize)
        }
        .ok_or(ProgramError::InvalidAccountData)?;

        self.remove_at_slot::<P>(idx)
    }

    pub fn remove_at_slot<P: Pod + Zeroable + std::fmt::Debug>(
        &mut self,
        pos: SlotIndex,
    ) -> Result<P, ProgramError> {
        msg!("removing element at {}", pos);
        let len = self.header.len;
        if pos == NIL || pos >= len {
            return Err(ProgramError::InvalidInstructionData);
        };

        let (prev, next, inner) = {
            let n: &DequeNode<P> = from_slot::<DequeNode<P>>(self.slots, pos)?;
            (n.prev, n.next, n.inner)
        };

        match prev {
            NIL => self.header.deque_head = next,
            prev => from_slot_mut::<DequeNode<P>>(self.slots, prev)?.next = next,
        }

        match next {
            NIL => self.header.deque_tail = prev,
            next => from_slot_mut::<DequeNode<P>>(self.slots, next)?.prev = prev,
        }

        self.header.len = self.header.len.saturating_sub(1);
        let mut free = Stack::<P>::new(self.slots, self.header.free_head);
        free.push_to_free(pos)?;
        self.header.free_head = free.get_head();
        Ok(inner)
    }

    // TODO: Fix generics for this later. It allows incorrect passing of Slabs. Currently
    // needs to be pod- tried a trait impl for next/prev/inner but it got too complex for POC.
    pub fn iter_indices_from_head<T: Pod>(&self) -> impl Iterator<Item = SlotIndex> + '_ {
        let start = (self.header.deque_head != NIL).then_some(self.header.deque_head);
        std::iter::successors(start, move |&i| {
            let maybe_node = from_slot::<DequeNode<T>>(self.slots, i).ok();
            let node = maybe_node?;
            (node.next != NIL).then_some(node.next)
        })
        .take(self.header.len as usize)
    }

    // TODO: Fix generics for this later. It allows incorrect passing of Slabs. Currently
    // needs to be pod- tried a trait impl for next/prev/inner but it got too complex for POC.
    pub fn iter_indices_from_tail<T: Pod>(&self) -> impl Iterator<Item = SlotIndex> + '_ {
        let start = (self.header.deque_tail != NIL).then_some(self.header.deque_tail);
        std::iter::successors(start, move |&i| {
            let maybe_node = from_slot::<DequeNode<T>>(self.slots, i).ok();
            let node = maybe_node?;
            (node.prev != NIL).then_some(node.prev)
        })
        .take(self.header.len as usize)
    }
}
