//! # no_std Memory Primitives
//!
//! In a `#![no_std]` environment, you have no standard library — only `core`.
//! These memory operation functions are the most fundamental building blocks in an OS kernel.
//! Functions like memcpy/memset in libc must be implemented by ourselves in bare-metal environments.
//!
//! ## Task
//!
//! Implement the following five functions:
//! - Only use the `core` crate, no `std`
//! - Do not call `core::ptr::copy`, `core::ptr::copy_nonoverlapping`, etc. (write your own loops)
//! - Handle edge cases correctly (n=0, overlapping memory regions, etc.)
//! - Pass all tests

// Force no_std in production; allow std in tests (cargo test framework requires it)
#![cfg_attr(not(test), no_std)]
#![allow(unused_variables)]

/// Copy `n` bytes from `src` to `dst`.
///
/// - `dst` and `src` must not overlap (use `my_memmove` for overlapping regions)
/// - Returns `dst`
///
/// # Safety
/// `dst` and `src` must each point to at least `n` bytes of valid memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 { // 这是裸指针
    // TODO: Implement memcpy
    // Hint: read bytes from src one by one and write to dst
    
    let mut  i = 0;
    // 从 0 开始进行操作，也就是 0 —— n-1
    while i < n {
        
        // 操作裸指针
        unsafe{
            let byte = src.add(i).read();
            dst.add(i).write(byte)
        }    

        i+=1;
    } 

    dst

}

/// Set `n` bytes starting at `dst` to the value `c`.
///
/// Returns `dst`.
///
/// # Safety
/// `dst` must point to at least `n` bytes of valid writable memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_memset(dst: *mut u8, c: u8, n: usize) -> *mut u8 {
    // TODO: Implement memset
    let mut i = 0;
    while i < n {
        // add() 返回一个新指针，原来的 dst 指针指向的位置是不变的！
        dst.add(i).write(c);
        i+=1;
    }
    dst
}

/// Copy `n` bytes from `src` to `dst`, correctly handling overlapping memory.
///
/// Returns `dst`.
///
/// # Safety
/// `dst` and `src` must each point to at least `n` bytes of valid memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // TODO: Implement memmove
    // Hint: when dst > src and regions overlap, copy backwards (from end to start)

    // 这里的提示说明了，两块区域可能会有重叠的情况，如果重叠的话，那么就要从后往前复制了
    let d = dst as usize;
    let s = src as usize; // 转为地址
    
    // 目标在源的后面，并且发生重叠了
    // s + n 对应的地址比 d 大，代表需要复制的 n 个元素和 d 有重叠的部分（判断是否有重叠）
    if d > s && d -s < n {
        // 此时应该反向复制元素
        let mut i = n;
        while i > 0 {
            // 先减一，索引是正确的
            i-=1;
            unsafe{
                dst.add(i).write(src.add(i).read());
            }
        }
        
    }else{
        // dst 在 src 的左侧 或 不重叠 -> 直接从前往后复制即可
        let mut i = 0;
        while i < n {
            
            unsafe{
                dst.add(i).write(src.add(i).read());
            }
            i+=1;
        }

    }

    dst
}

/// Return the length of a null-terminated byte string, excluding the trailing null.
///
/// # Safety
/// `s` must point to a valid null-terminated byte string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_strlen(s: *const u8) -> usize {
    // TODO: Implement strlen
    let mut len = 0;
    loop{
        // 解引用裸指针，即便函数 unsafe 了，在操作的时候也必须使用 unsafe 块
        let byte = unsafe{
            s.add(len).read()
        };
        if byte == 0 {
            break len;
        }else{
            len+=1;
        }
    }
    
}

/// Compare two null-terminated byte strings.
///
/// Returns:
/// - `0`  : strings are equal
/// - `< 0`: `s1` is lexicographically less than `s2`
/// - `> 0`: `s1` is lexicographically greater than `s2`
///
/// # Safety
/// `s1` and `s2` must each point to a valid null-terminated byte string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn my_strcmp(s1: *const u8, s2: *const u8) -> i32 {
    // TODO: Implement strcmp
    
    let mut i = 0;

    loop {
        unsafe {
            // 读取两个字符串当前位置的字节
            let byte1 = s1.add(i).read();
            let byte2 = s2.add(i).read();

            // 遇到第一个不同的字节，立即返回
            if byte1 < byte2 {
                return -1;
            }

            if byte1 > byte2 {
                return 1;
            }

            // 两个字节相同，并且都是 '\0'
            // 说明两个字符串完全相同
            if byte1 == 0 {
                return 0;
            }
        }

        // 当前字节相同且不为 '\0'，继续比较
        i += 1;
    }
    

}

// ============================================================
// Tests (std is available under #[cfg(test)])
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memcpy_basic() {
        let src = [1u8, 2, 3, 4, 5];
        let mut dst = [0u8; 5];
        unsafe { my_memcpy(dst.as_mut_ptr(), src.as_ptr(), 5) };
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memcpy_zero_len() {
        let src = [0xFFu8; 4];
        let mut dst = [0u8; 4];
        unsafe { my_memcpy(dst.as_mut_ptr(), src.as_ptr(), 0) };
        assert_eq!(dst, [0u8; 4]);
    }

    #[test]
    fn test_memset_basic() {
        let mut buf = [0u8; 8];
        unsafe { my_memset(buf.as_mut_ptr(), 0xAB, 8) };
        assert!(buf.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn test_memset_partial() {
        let mut buf = [0u8; 8];
        unsafe { my_memset(buf.as_mut_ptr(), 0xFF, 4) };
        assert_eq!(&buf[..4], &[0xFF; 4]);
        assert_eq!(&buf[4..], &[0x00; 4]);
    }

    #[test]
    fn test_memmove_no_overlap() {
        let src = [1u8, 2, 3, 4];
        let mut dst = [0u8; 4];
        unsafe { my_memmove(dst.as_mut_ptr(), src.as_ptr(), 4) };
        assert_eq!(dst, src);
    }

    #[test]
    fn test_memmove_overlap_forward() {
        // Copy buf[0..4] to buf[1..5], shifting right by 1
        let mut buf = [1u8, 2, 3, 4, 5];
        unsafe { my_memmove(buf.as_mut_ptr().add(1), buf.as_ptr(), 4) };
        assert_eq!(buf, [1, 1, 2, 3, 4]);
    }

    #[test]
    fn test_strlen_basic() {
        let s = b"hello\0";
        assert_eq!(unsafe { my_strlen(s.as_ptr()) }, 5);
    }

    #[test]
    fn test_strlen_empty() {
        let s = b"\0";
        assert_eq!(unsafe { my_strlen(s.as_ptr()) }, 0);
    }

    #[test]
    fn test_strcmp_equal() {
        let a = b"hello\0";
        let b = b"hello\0";
        assert_eq!(unsafe { my_strcmp(a.as_ptr(), b.as_ptr()) }, 0);
    }

    #[test]
    fn test_strcmp_less() {
        let a = b"abc\0";
        let b = b"abd\0";
        assert!(unsafe { my_strcmp(a.as_ptr(), b.as_ptr()) } < 0);
    }

    #[test]
    fn test_strcmp_greater() {
        let a = b"abd\0";
        let b = b"abc\0";
        assert!(unsafe { my_strcmp(a.as_ptr(), b.as_ptr()) } > 0);
    }
}
