use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, system_instruction, sysvar::Sysvar,
};

use crate::{
    state::{get_deque_address, Deque, DequeNode, MarketEscrow, Stack, HEADER_FIXED_SIZE},
    PROGRAM_ID_PUBKEY,
};

/// The ordinal `sector` index in the slab of bytes dedicated to inner data for a type.
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
    let i = byte_offset;
    bytemuck::try_from_bytes(&data[i..i + std::mem::size_of::<T>()])
        .map_err(|_| ProgramError::InvalidAccountData)
}

#[inline(always)]
pub fn from_slab_bytes_mut<T: Slab>(
    data: &mut [u8],
    byte_offset: usize,
) -> Result<&mut T, ProgramError> {
    let i = byte_offset;
    bytemuck::try_from_bytes_mut(&mut data[i..i + std::mem::size_of::<T>()])
        .map_err(|_| ProgramError::InvalidAccountData)
}

#[inline(always)]
pub fn from_sector_idx<T: Slab>(sectors: &[u8], idx: SectorIndex) -> Result<&T, ProgramError> {
    if idx == NIL {
        return Err(ProgramError::InvalidAccountData);
    }
    let stride = size_of::<T>();
    let start = (idx as usize)
        .checked_mul(stride)
        .ok_or(ProgramError::InvalidAccountData)?;
    from_slab_bytes(sectors, start)
}

#[inline(always)]
pub fn from_sector_idx_mut<T: Slab>(
    sectors: &mut [u8],
    idx: SectorIndex,
) -> Result<&mut T, ProgramError> {
    if idx == NIL {
        return Err(ProgramError::InvalidAccountData);
    }
    let stride = size_of::<T>();
    let start = (idx as usize)
        .checked_mul(stride)
        .ok_or(ProgramError::InvalidAccountData)?;
    from_slab_bytes_mut(sectors, start)
}

pub fn log_bytes(bytes: &[u8]) {
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    msg!(&hex);
}

#[inline(always)]
pub fn check_owned_and_writable(account: &AccountInfo) -> ProgramResult {
    if account.owner.as_ref() != PROGRAM_ID_PUBKEY.as_ref() {
        msg!("account not owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if !account.is_writable {
        msg!("account not writable");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}

#[inline(always)]
pub fn check_derivations_and_get_bump(
    deque_account: &AccountInfo,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
) -> Result<u8, ProgramError> {
    let (deque_pub, deque_bump) = get_deque_address(base_mint, quote_mint);
    if deque_pub.as_ref() != deque_account.key.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(deque_bump)
}

#[inline(always)]
pub fn inline_resize<'a, 'info>(
    deque_account: &'a AccountInfo<'info>,
    payer_account: &'a AccountInfo<'info>,
    system_program: &'a AccountInfo<'info>,
    num_sectors: u16,
) -> ProgramResult {
    if num_sectors < 1 {
        return Err(ProgramError::InvalidArgument);
    }
    check_owned_and_writable(deque_account)?;

    let deque_data = deque_account.try_borrow_mut_data()?;
    let current_size = deque_data.len();
    let current_lamports = deque_account.lamports();
    let new_account_space = current_size + SECTOR_SIZE * (num_sectors as usize);
    let new_lamports_required = Rent::get()?.minimum_balance(new_account_space);
    let lamports_diff = new_lamports_required.saturating_sub(current_lamports);

    drop(deque_data);

    if lamports_diff > 0 {
        invoke(
            &system_instruction::transfer(payer_account.key, deque_account.key, lamports_diff),
            &[
                payer_account.clone(),
                deque_account.clone(),
                system_program.clone(),
            ],
        )?;
    }

    // "Memory used to grow is already zero-initialized upon program entrypoint and re-zeroing it wastes compute units."
    // See: https://solana.com/developers/courses/program-optimization/program-architecture#data-optimization
    deque_account.realloc(new_account_space, false)?;

    // Now chain the old sectors to the new sectors in the stack of free nodes.
    let mut deque_data = deque_account.data.borrow_mut();
    let deque = Deque::new_from_bytes_unchecked(&mut deque_data)?;

    let curr_n_sectors = (current_size - HEADER_FIXED_SIZE) / SECTOR_SIZE;
    let new_n_sectors = curr_n_sectors + num_sectors as usize;

    let mut free = Stack::<MarketEscrow>::new(deque.sectors, deque.header.free_head);
    for i in curr_n_sectors..new_n_sectors {
        free.push_to_free(i as SectorIndex)?;
    }
    deque.header.free_head = free.get_head();

    drop(deque_data);

    Ok(())
}
