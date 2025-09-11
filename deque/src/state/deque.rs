use bytemuck::{Pod, Zeroable};
use solana_program::{msg, program_error::ProgramError, pubkey::Pubkey};
use static_assertions::const_assert_eq;

use crate::{
    state::{DequeHeader, DequeType, MarketEscrow, Stack, StackNode, HEADER_FIXED_SIZE},
    utils::{from_sector_idx, from_sector_idx_mut, from_slab_bytes_mut, SectorIndex, Slab, NIL},
};

#[derive(Clone, Copy, Debug, Zeroable)]
#[repr(C)]
pub struct DequeNode<T> {
    // The inner payload bytes.
    pub inner: T,
    pub prev: SectorIndex,
    pub next: SectorIndex,
}

// Ensure that deque and stack nodes are the same size, regardless of type.
const_assert_eq!(size_of::<DequeNode<u8>>(), size_of::<StackNode<u8>>());
const_assert_eq!(size_of::<DequeNode<u32>>(), size_of::<StackNode<u32>>());
const_assert_eq!(size_of::<DequeNode<u64>>(), size_of::<StackNode<u64>>());
const_assert_eq!(
    size_of::<DequeNode<[u8; 7]>>(),
    size_of::<StackNode<[u8; 7]>>()
);

unsafe impl<T: Pod> Pod for DequeNode<T> {}

impl<T: Pod> Slab for DequeNode<T> {}

pub struct Deque<'a> {
    pub header: &'a mut DequeHeader,
    // Either StackNode<T> or DequeNode<T>
    pub sectors: &'a mut [u8],
}

impl<'a> Deque<'a> {
    /// Construct a new, empty Deque, given an existing header and the deque type input.
    /// It's assumed that the data has already been allocated and aligned properly to the number
    /// of sectors.
    pub fn init_deque_account(
        data: &mut [u8],
        deque_type: DequeType,
        num_sectors: u16,
        deque_bump: u8,
        vault_ctx: (&Pubkey, u8),
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
    ) -> Result<(), ProgramError> {
        if data.len() < HEADER_FIXED_SIZE {
            return Err(ProgramError::InvalidAccountData);
        }

        let (vault, vault_bump) = vault_ctx;

        let mut deque = Deque::new_from_bytes(data)?;
        // Write a new empty header to the `deque.header`
        *deque.header = DequeHeader::new_empty(
            deque_type, vault, deque_bump, vault_bump, base_mint, quote_mint,
        );

        let sector_size = deque_type.sector_size();
        debug_assert_eq!(deque.sectors.len() % sector_size, 0);
        debug_assert_eq!(deque.sectors.len(), (num_sectors as usize) * sector_size);

        let space_needed = (num_sectors as usize)
            .checked_mul(sector_size)
            .ok_or(ProgramError::InvalidAccountData)?;
        if deque.sectors.len() < space_needed {
            return Err(ProgramError::InvalidAccountData);
        }

        match deque_type {
            DequeType::U32 => deque.init_free_stack::<u32>(num_sectors as usize)?,
            DequeType::U64 => deque.init_free_stack::<u64>(num_sectors as usize)?,
            DequeType::Market => deque.init_free_stack::<MarketEscrow>(num_sectors as usize)?,
        }

        Ok(())
    }

    pub fn init_free_stack<T: Pod + Zeroable>(
        &mut self,
        num_sectors: usize,
    ) -> Result<(), ProgramError> {
        let mut stack = Stack::<T>::new(self.sectors, self.header.free_head);
        for s in (0..num_sectors).rev() {
            stack.push_to_free(s as SectorIndex)?;
        }
        self.header.free_head = stack.get_head();
        Ok(())
    }

    /// Construct a Deque from an existing byte vector- assumed to be well-formed.
    pub fn new_from_bytes(data: &'a mut [u8]) -> Result<Self, ProgramError> {
        let (header_slab, sectors) = data.split_at_mut(HEADER_FIXED_SIZE);
        let header = from_slab_bytes_mut::<DequeHeader>(header_slab, 0_usize)?;
        Ok(Self { header, sectors })
    }

    pub fn as_free_mut<P: Pod>(
        &mut self,
        idx: SectorIndex,
    ) -> Result<&mut StackNode<P>, ProgramError> {
        let free_node = from_sector_idx_mut::<StackNode<P>>(self.sectors, idx)?;
        Ok(free_node)
    }

    pub fn as_deque_mut<P: Pod>(
        &mut self,
        idx: SectorIndex,
    ) -> Result<&mut DequeNode<P>, ProgramError> {
        let deque_node = from_sector_idx_mut::<DequeNode<P>>(self.sectors, idx)?;
        Ok(deque_node)
    }

    pub fn push_front<P: Pod + Zeroable + std::fmt::Debug>(
        &mut self,
        value: P,
    ) -> Result<SectorIndex, ProgramError> {
        msg!("pushing {:#?} to front", value);
        let mut free = Stack::<P>::new(self.sectors, self.header.free_head);
        let new_idx = free.remove_from_free()?;
        self.header.free_head = free.get_head();
        if new_idx == NIL {
            return Err(ProgramError::AccountDataTooSmall);
        }

        let head = self.header.deque_head;
        let n: &mut DequeNode<P> = from_sector_idx_mut(self.sectors, new_idx)?;
        *n = DequeNode {
            inner: value,
            prev: NIL,
            next: head,
        };

        match head {
            NIL => self.header.deque_tail = new_idx,
            head => from_sector_idx_mut::<DequeNode<P>>(self.sectors, head)?.prev = new_idx,
        }

        self.header.deque_head = new_idx;
        self.header.len = self.header.len.saturating_add(1);
        Ok(new_idx)
    }

    pub fn push_back<P: Pod + Zeroable>(&mut self, value: P) -> Result<SectorIndex, ProgramError> {
        let mut free = Stack::<P>::new(self.sectors, self.header.free_head);
        let new_idx = free.remove_from_free()?;
        self.header.free_head = free.get_head();
        if new_idx == NIL {
            return Err(ProgramError::InvalidAccountData);
        }

        let tail = self.header.deque_tail;
        let n: &mut DequeNode<P> = from_sector_idx_mut(self.sectors, new_idx)?;
        *n = DequeNode {
            inner: value,
            prev: tail,
            next: NIL,
        };

        match tail {
            NIL => self.header.deque_head = new_idx,
            tail => from_sector_idx_mut::<DequeNode<P>>(self.sectors, tail)?.next = new_idx,
        }

        self.header.deque_tail = new_idx;
        self.header.len = self.header.len.saturating_add(1);
        Ok(new_idx)
    }

    pub fn remove<P: Pod + Zeroable + std::fmt::Debug>(
        &mut self,
        pos: SectorIndex,
    ) -> Result<P, ProgramError> {
        let len = self.header.len;
        if pos >= len {
            return Err(ProgramError::InvalidArgument);
        }

        // Pick the closer direction, grab the sector index
        let idx = if pos <= len / 2 {
            self.iter_indices_from_head::<P>().nth(pos as usize)
        } else {
            self.iter_indices_from_tail::<P>()
                .nth((len - 1 - pos) as usize)
        }
        .ok_or(ProgramError::InvalidAccountData)?;

        self.remove_at_sector::<P>(idx)
    }

    pub fn remove_at_sector<P: Pod + Zeroable + std::fmt::Debug>(
        &mut self,
        pos: SectorIndex,
    ) -> Result<P, ProgramError> {
        msg!("removing element at {}", pos);
        let len = self.header.len;
        if pos == NIL || pos >= len {
            return Err(ProgramError::InvalidInstructionData);
        };

        let (prev, next, inner) = {
            let n: &DequeNode<P> = from_sector_idx::<DequeNode<P>>(self.sectors, pos)?;
            (n.prev, n.next, n.inner)
        };

        match prev {
            NIL => self.header.deque_head = next,
            prev => from_sector_idx_mut::<DequeNode<P>>(self.sectors, prev)?.next = next,
        }

        match next {
            NIL => self.header.deque_tail = prev,
            next => from_sector_idx_mut::<DequeNode<P>>(self.sectors, next)?.prev = prev,
        }

        self.header.len = self.header.len.saturating_sub(1);
        let mut free = Stack::<P>::new(self.sectors, self.header.free_head);
        free.push_to_free(pos)?;
        self.header.free_head = free.get_head();
        Ok(inner)
    }

    // TODO: Fix generics for this later. It allows incorrect passing of Slabs. Currently
    // needs to be pod- tried a trait impl for next/prev/inner but it got too complex for POC.
    pub fn iter_indices_from_head<T: Pod>(&self) -> impl Iterator<Item = SectorIndex> + '_ {
        let start = (self.header.deque_head != NIL).then_some(self.header.deque_head);
        std::iter::successors(start, move |&i| {
            let maybe_node = from_sector_idx::<DequeNode<T>>(self.sectors, i).ok();
            let node = maybe_node?;
            (node.next != NIL).then_some(node.next)
        })
        .take(self.header.len as usize)
    }

    // TODO: Fix generics for this later. It allows incorrect passing of Slabs. Currently
    // needs to be pod- tried a trait impl for next/prev/inner but it got too complex for POC.
    pub fn iter_indices_from_tail<T: Pod>(&self) -> impl Iterator<Item = SectorIndex> + '_ {
        let start = (self.header.deque_tail != NIL).then_some(self.header.deque_tail);
        std::iter::successors(start, move |&i| {
            let maybe_node = from_sector_idx::<DequeNode<T>>(self.sectors, i).ok();
            let node = maybe_node?;
            (node.prev != NIL).then_some(node.prev)
        })
        .take(self.header.len as usize)
    }
}
