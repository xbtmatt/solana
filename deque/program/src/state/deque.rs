use bytemuck::{Pod, Zeroable};
use solana_program::{entrypoint::ProgramResult, msg, program_error::ProgramError, pubkey::Pubkey};
use static_assertions::const_assert_eq;

use crate::{
    shared::error::DequeError,
    state::{DequeHeader, MarketEscrow, Stack, StackNode, HEADER_FIXED_SIZE},
    utils::{
        from_sector_idx, from_sector_idx_mut, from_slab_bytes_mut, SectorIndex, Slab, NIL,
        SECTOR_SIZE,
    },
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
const_assert_eq!(
    size_of::<DequeNode<MarketEscrow>>(),
    size_of::<StackNode<MarketEscrow>>()
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
        num_sectors: u16,
        deque_bump: u8,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
    ) -> ProgramResult {
        if data.len() < HEADER_FIXED_SIZE {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut deque = Deque::new_from_bytes_unchecked(data)?;
        // Write a new empty header to the `deque.header`
        *deque.header = DequeHeader::new_empty(deque_bump, base_mint, quote_mint);

        debug_assert_eq!(deque.sectors.len() % SECTOR_SIZE, 0);
        debug_assert_eq!(deque.sectors.len(), (num_sectors as usize) * SECTOR_SIZE);

        deque.init_free_stack::<MarketEscrow>(num_sectors as usize)?;

        Ok(())
    }

    pub fn init_free_stack<T: Pod + Zeroable>(&mut self, num_sectors: usize) -> ProgramResult {
        let mut stack = Stack::<T>::new(self.sectors, self.header.free_head);
        for s in (0..num_sectors).rev() {
            stack.push_to_free(s as SectorIndex)?;
        }
        self.header.free_head = stack.get_head();
        Ok(())
    }

    pub fn get_capacity(&self) -> u32 {
        (self.sectors.len() / SECTOR_SIZE) as u32
    }

    /// Construct a Deque from an existing byte vector without checking the header's discriminant.
    pub fn new_from_bytes_unchecked(data: &'a mut [u8]) -> Result<Self, ProgramError> {
        let (header_slab, sectors) = data.split_at_mut(HEADER_FIXED_SIZE);
        let header = from_slab_bytes_mut::<DequeHeader>(header_slab, 0_usize)?;
        Ok(Self { header, sectors })
    }

    /// Construct a Deque from an existing byte vector and check the header's discriminant.
    pub fn new_from_bytes(data: &'a mut [u8]) -> Result<Self, ProgramError> {
        let (header_slab, sectors) = data.split_at_mut(HEADER_FIXED_SIZE);
        let header = from_slab_bytes_mut::<DequeHeader>(header_slab, 0_usize)?;
        header.verify_discriminant()?;
        Ok(Self { header, sectors })
    }

    pub fn push_front<P: Pod + Zeroable + core::fmt::Debug>(
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

    /// Remove by an ordinal/logical index in the deque.
    /// That is, remove at the *logical* index in the deque, not the *physical* index in memory.
    pub fn remove_at_logical_idx<P: Pod + Zeroable + core::fmt::Debug>(
        &mut self,
        logical_idx: u32,
    ) -> Result<P, ProgramError> {
        let len = self.header.len;
        if logical_idx >= len {
            return Err(DequeError::OutOfBounds.into());
        }

        // Pick the closer direction, grab the sector index
        let idx = if logical_idx <= len / 2 {
            self.iter_indices::<P>().nth(logical_idx as usize)
        } else {
            self.iter_indices_rev::<P>()
                .nth((len - 1 - logical_idx) as usize)
        }
        .ok_or(ProgramError::InvalidAccountData)?;

        self.remove_at_sector_idx::<P>(idx)
    }

    pub fn remove_at_sector_idx<P: Pod + Zeroable + core::fmt::Debug>(
        &mut self,
        idx: SectorIndex,
    ) -> Result<P, ProgramError> {
        msg!("removing element at {}", idx);
        if idx == NIL {
            return Err(ProgramError::InvalidInstructionData);
        };

        let (prev, next, inner) = {
            let n: &DequeNode<P> = from_sector_idx::<DequeNode<P>>(self.sectors, idx)?;
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
        msg!("Header len just updated TO: {}", self.header.len);
        let mut free = Stack::<P>::new(self.sectors, self.header.free_head);
        free.push_to_free(idx)?;
        self.header.free_head = free.get_head();
        Ok(inner)
    }

    pub fn iter_nodes<T: Pod>(&self) -> impl Iterator<Item = (&T, SectorIndex)> + '_ {
        self.iter_indices::<T>().filter_map(move |i| {
            from_sector_idx::<DequeNode<T>>(self.sectors, i)
                .ok()
                .map(|node| (&node.inner, i))
        })
    }

    // TODO: Fix generics for this later. It allows incorrect passing of Slabs. Currently
    // needs to be pod- tried a trait impl for next/prev/inner but it got too complex for POC.
    pub fn iter_indices<T: Pod>(&self) -> impl Iterator<Item = SectorIndex> + '_ {
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
    pub fn iter_indices_rev<T: Pod>(&self) -> impl Iterator<Item = SectorIndex> + '_ {
        let start = (self.header.deque_tail != NIL).then_some(self.header.deque_tail);
        std::iter::successors(start, move |&i| {
            let maybe_node = from_sector_idx::<DequeNode<T>>(self.sectors, i).ok();
            let node = maybe_node?;
            (node.prev != NIL).then_some(node.prev)
        })
        .take(self.header.len as usize)
    }
}

#[cfg(not(target_os = "solana"))]
impl<'a> core::fmt::Debug for Deque<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let items: Vec<_> = self
            .iter_indices::<MarketEscrow>()
            .filter_map(|idx| {
                from_sector_idx::<DequeNode<MarketEscrow>>(self.sectors, idx)
                    .ok()
                    .map(|node| node.inner)
            })
            .collect();

        f.debug_struct("Deque")
            .field("len", &self.header.len)
            .field("deque_head", &self.header.deque_head)
            .field("deque_tail", &self.header.deque_tail)
            .field("free_head", &self.header.free_head)
            .field("items", &items)
            .finish()
    }
}
