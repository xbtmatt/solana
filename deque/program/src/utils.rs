use core::mem::MaybeUninit;

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, system_instruction, sysvar::Sysvar,
};

use crate::{
    seeds,
    shared::error::{DequeError, DequeProgramResult},
    state::{Deque, DequeNode, MarketEscrow, Stack, DEQUE_HEADER_SIZE},
    validation::system_program::SystemProgramInfo,
};

/// The physical `sector` index in the slab of bytes dedicated to inner data for a type.
/// That is, to get the raw bytes offset, it is multiplied by the sector type's sector size.
pub type SectorIndex = u32;
pub const NIL: SectorIndex = SectorIndex::MAX;
pub const SECTOR_SIZE: usize = size_of::<DequeNode<MarketEscrow>>();

/// Below is taken directly from:
/// https://github.com/solana-program/libraries/blob/main/pod/src/primitives.rs
///
/// The standard `bool` is not a `Pod`, define a replacement that is
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(transparent)]
pub struct PodBool(pub u8);
impl PodBool {
    pub const fn from_bool(b: bool) -> Self {
        Self(if b { 1 } else { 0 })
    }
}

impl From<bool> for PodBool {
    fn from(b: bool) -> Self {
        Self::from_bool(b)
    }
}

/// Marker trait to narrow an account data's Pod-based outer Node<T> type (used as a slice of
/// mutable bytes: &mut [u8]) so that T can't also be passed.
pub trait Slab: bytemuck::Pod {}

#[inline(always)]
pub fn from_slab_bytes<T: Slab>(data: &[u8], byte_offset: usize) -> Result<&T, ProgramError> {
    // TODO: Optimize this to use `get_unchecked` with a prior safety check on the data/buffer size.
    let size = core::mem::size_of::<T>();
    let end = byte_offset
        .checked_add(size)
        .ok_or(DequeError::ArithmetricError)?;
    let bytes = data.get(byte_offset..end).ok_or(DequeError::OutOfBounds)?;
    bytemuck::try_from_bytes(bytes).map_err(|_| DequeError::MalformedSlab.into())
}

#[inline(always)]
pub fn from_slab_bytes_mut<T: Slab>(
    data: &mut [u8],
    byte_offset: usize,
) -> Result<&mut T, ProgramError> {
    // TODO: Optimize this to use `get_unchecked` with a prior safety check on the data/buffer size.
    let size = core::mem::size_of::<T>();
    let end = byte_offset
        .checked_add(size)
        .ok_or(DequeError::ArithmetricError)?;
    let bytes = data
        .get_mut(byte_offset..end)
        .ok_or(DequeError::OutOfBounds)?;
    bytemuck::try_from_bytes_mut(bytes).map_err(|_| DequeError::MalformedSlab.into())
}

#[inline(always)]
pub fn from_sector_idx<T: Slab>(sectors: &[u8], idx: SectorIndex) -> Result<&T, ProgramError> {
    if idx == NIL {
        return Err(DequeError::InvalidSectorIndex.into());
    }
    let stride = size_of::<T>();
    let start = (idx as usize)
        .checked_mul(stride)
        .ok_or(DequeError::ArithmetricError)?;
    from_slab_bytes(sectors, start)
}

#[inline(always)]
pub fn from_sector_idx_mut<T: Slab>(
    sectors: &mut [u8],
    idx: SectorIndex,
) -> Result<&mut T, ProgramError> {
    if idx == NIL {
        return Err(DequeError::InvalidSectorIndex.into());
    }
    let stride = size_of::<T>();
    let start = (idx as usize)
        .checked_mul(stride)
        .ok_or(DequeError::ArithmetricError)?;
    from_slab_bytes_mut(sectors, start)
}

pub fn log_bytes(bytes: &[u8]) {
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    msg!(&hex);
}

#[inline(always)]
pub fn check_owned_and_writable(account: &AccountInfo) -> DequeProgramResult {
    if account.owner.as_ref() != crate::ID.as_ref() {
        Err(DequeError::AccountNotOwnedByProgram)
    } else if !account.is_writable {
        Err(DequeError::AccountIsNotWritable)
    } else {
        Ok(())
    }
}

#[inline(always)]
pub fn check_derivations_and_get_bump(
    deque_account: &AccountInfo,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
) -> Result<u8, DequeError> {
    // TODO: Determine if this is necessary. It's possible the bump can be passed and then the
    // attempted invoke signed should just not work if it's inaccurate..?
    let (deque_pub, deque_bump) = seeds::market::find_market_address(base_mint, quote_mint);
    if deque_pub.as_ref() != deque_account.key.as_ref() {
        return Err(DequeError::InvalidPDA);
    }

    Ok(deque_bump)
}

pub fn fund_then_resize<'a, 'info>(
    target: &'a AccountInfo<'info>,
    payer: &'a AccountInfo<'info>,
    system_program: &'a AccountInfo<'info>,
    additional_space: usize,
) -> DequeProgramResult {
    check_owned_and_writable(target)?;

    let current_size = target.data_len();
    let current_lamports = target.lamports();
    let target_space = current_size + additional_space;
    let target_lamports_required = Rent::get()
        .or(Err(DequeError::RentGetError))?
        .minimum_balance(target_space);
    let lamports_diff = target_lamports_required.saturating_sub(current_lamports);

    if lamports_diff > 0 {
        invoke(
            &system_instruction::transfer(payer.key, target.key, lamports_diff),
            &[payer.clone(), target.clone(), system_program.clone()],
        )
        .or(Err(DequeError::TransferError))?;
    }

    // "Memory used to grow is already zero-initialized upon program entrypoint and re-zeroing it wastes compute units."
    // See: https://solana.com/developers/courses/program-optimization/program-architecture#data-optimization
    target
        .realloc(target_space, false)
        .or(Err(DequeError::ReallocError))?;

    Ok(())
}

#[inline(always)]
pub fn inline_deque_resize<'a, 'info>(
    deque_account: &'a AccountInfo<'info>,
    payer_account: &'a AccountInfo<'info>,
    system_program: &'a AccountInfo<'info>,
    num_sectors: u16,
) -> ProgramResult {
    if num_sectors < 1 {
        return Err(DequeError::MustBeGreaterThanOne.into());
    }

    let current_size = deque_account.data_len();

    fund_then_resize(
        deque_account,
        payer_account,
        system_program,
        SECTOR_SIZE * (num_sectors as usize),
    )?;

    // Now chain the old sectors to the new sectors in the stack of free nodes.
    let mut deque_data = deque_account.data.borrow_mut();
    let deque = Deque::from_bytes_unchecked(&mut deque_data)?;

    let curr_n_sectors = (current_size - DEQUE_HEADER_SIZE) / SECTOR_SIZE;
    let new_n_sectors = curr_n_sectors + num_sectors as usize;

    let mut free = Stack::<MarketEscrow>::new(deque.sectors, deque.header.free_head);
    for i in curr_n_sectors..new_n_sectors {
        free.push_to_free(i as SectorIndex)?;
    }
    deque.header.free_head = free.get_head();

    drop(deque_data);

    Ok(())
}

pub const UNINIT_BYTE: MaybeUninit<u8> = MaybeUninit::uninit();

/// A byte-by-byte copy from one slice to another without having to zero init on the `dst` slice.
/// This is more explicit and less efficient than `sol_memcpy_` (in non-solana land it would be
/// `copy_from_nonoverlapping`), but it removes the risk of undefined behavior since the iterator
/// makes it impossible to write past the end of `dst`.
///
/// While it's not technically undefined behavior, a partially written to `dst` will result in
/// unexpected results. Ensure that both slices are at least the expected length.
///
/// # Example
/// ```
/// use core::mem::MaybeUninit;
///
/// const UNINIT_BYTE: MaybeUninit<u8> = MaybeUninit::uninit();
///
/// // Build a simple 5-byte message: [type, id, id, id, id]
/// let mut message = [UNINIT_BYTE; 5];
/// let message_type: u8 = 3;
/// let user_id: u32 = 1234;
///
/// // Write message type at offset 0
/// write_bytes(&mut message[0..1], &[message_type]);
/// // Write user ID at offset 1..5
/// write_bytes(&mut message[1..5], &user_id.to_le_bytes());
///
/// // This confines the `unsafe` behavior to the raw pointer cast back to a slice, which is now
/// // safe because all 5 bytes were explicitly written to.
/// let final_message: &[u8] = unsafe {
///     core::slice::from_raw_parts(message.as_ptr() as *const u8, 5)
/// };
/// ```
///
/// From pinocchio's `[no_std]` library:
/// https://github.com/anza-xyz/pinocchio/blob/3044aaf5ea7eac01adc754d4bdf93c21c6e54d42/programs/token/src/lib.rs#L13`
#[inline(always)]
pub fn write_bytes(dst: &mut [MaybeUninit<u8>], src: &[u8]) {
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        d.write(*s);
    }
}
