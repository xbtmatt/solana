use solana_program::program_error::ProgramError;

use crate::syscalls::sol_memcpy_;

const U16_BYTES: usize = core::mem::size_of::<u16>();

#[inline(always)]
pub fn unpack_u16(instruction_data: &[u8]) -> Result<u16, ProgramError> {
    if instruction_data.len() >= U16_BYTES {
        // SAFETY: `instruction_data` is at least `U16_BYTES`.
        Ok(unsafe { u16::from_le_bytes(*(instruction_data.as_ptr() as *const [u8; U16_BYTES])) })
    } else {
        Err(ProgramError::InvalidInstructionData)
    }
}

const U64_BYTES: usize = core::mem::size_of::<u64>();

#[inline(always)]
pub fn unpack_u64(instruction_data: &[u8]) -> Result<u64, ProgramError> {
    if instruction_data.len() >= U64_BYTES {
        // SAFETY: `instruction_data` is at least `U64_BYTES`.
        Ok(unsafe { u64::from_le_bytes(*(instruction_data.as_ptr() as *const [u8; U64_BYTES])) })
    } else {
        Err(ProgramError::InvalidInstructionData)
    }
}

/// Append a slice of bytes to a heap-allocated Vec using the sol_memcpy syscall.
/// # Safety
// - dst.capacity() - dst.len() >= src.len()
// - `src` does not overlap `dst`
// - `dst` is writeable and `src` is readable
#[inline(always)]
pub unsafe fn vec_append_bytes(dst: &mut Vec<u8>, src: &[u8]) {
    unsafe {
        // Get the pointer to the allocated but maybe uninitialized bytes at the end.
        let dst_ptr = dst.as_mut_ptr().add(dst.len());
        let src_ptr = src.as_ptr();
        // And write the bytes at `src` to it.
        sol_memcpy_(dst_ptr, src_ptr, src.len() as u64);
        // Publish the new bytes by manually setting the length.
        dst.set_len(dst.len() + src.len());
    }
}

pub mod tests {
    #[test]
    pub fn unpack_tests() {
        use crate::pack::{unpack_u16, unpack_u64};

        let value = 12345u16;
        assert_eq!(
            unpack_u16(&value.to_le_bytes()).expect("Should unpack"),
            value
        );

        let value = 123456789u64;
        assert_eq!(
            unpack_u64(&value.to_le_bytes()).expect("Should unpack"),
            value
        );
    }
}
