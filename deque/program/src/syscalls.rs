// Copied/heavily based on the implementation at:
// https://github.com/anza-xyz/pinocchio/blob/7ccb7ba4d02c410dc34000713c4803e6922a06da/sdk/log/crate/src/logger.rs#L4

#[cfg(all(target_os = "solana", not(target_feature = "static-syscalls")))]
// Syscalls provided by the SVM runtime (SBPFv0, SBPFv1 and SBPFv2).
extern "C" {
    pub fn sol_memcpy_(dst: *mut u8, src: *const u8, num_bytes: u64);
}

#[cfg(all(target_os = "solana", target_feature = "static-syscalls"))]
// Syscalls provided by the SVM runtime (SBPFv3 and newer).
pub(crate) fn sol_memcpy_(dst: *mut u8, src: *const u8, num_bytes: u64) {
    let syscall: extern "C" fn(*mut u8, *const u8, u64) =
        unsafe { core::mem::transmute(1904002211u64) }; // murmur32 hash of "sol_memcpy_"
    syscall(dst, src, num_bytes)
}

#[cfg(not(target_os = "solana"))]
#[allow(dead_code)]
pub(crate) fn sol_memcpy_(dst: *mut u8, src: *const u8, num_bytes: u64) {
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, num_bytes as usize);
    }
}
