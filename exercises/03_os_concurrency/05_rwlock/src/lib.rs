//! # Read-Write Lock (Writer-Priority)
//!
//! In this exercise, you will implement a **writer-priority** read-write lock from scratch using atomics.
//! Multiple readers may hold the lock concurrently; a writer holds it exclusively.
//!
//! **Note:** Rust's standard library already provides [`std::sync::RwLock`]. This exercise implements
//! a minimal version for learning the protocol and policy without using the standard one.
//!
//! ## Common policies for read-write locks
//! Different implementations can give different **priority** when both readers and writers are waiting:
//!
//! - **Reader-priority (读者优先)**: New readers are allowed to enter while a writer is waiting, so writers
//!   may be starved if readers keep arriving.
//! - **Writer-priority (写者优先)**: Once a writer is waiting, no new readers are admitted until that writer
//!   has run; this exercise implements this policy.
//! - **Read-write fair (读写公平)**: Requests are served in a fair order (e.g. FIFO or round-robin), so
//!   neither readers nor writers are systematically starved.
//!
//! ## Key Concepts
//! - **Readers**: share access; many threads can hold a read lock at once.
//! - **Writer**: exclusive access; only one writer, and no readers while the writer holds the lock.
//! - **Writer-priority (this implementation)**: when at least one writer is waiting, new readers block
//!   until the writer runs.
//!
//! ## State (single atomic)
//! We use one `AtomicU32`: low bits = reader count, two flags = writer holding / writer waiting.
//! All logic is implemented with compare_exchange and load/store; no use of `std::sync::RwLock`.

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicU32, Ordering};


// 读写锁位分段
// 写者优先
// + -------- + --------------------- + ------------------ +
// |  bit 31  |  bit 30 ~  16         |   bit 15 ~ 0       |
// |  WRITER  |  writer_count         |    reader_count    |
// |  HOLD    |                       |                    |
// + -------- + --------------------- + ------------------ + 

const WRITER_HOLDING: u32 = 1 << 31;
// 读者数量的掩码和最大值
const READER_MASK: u32 = (1 << 16) - 1;
// 等待写的数量的掩码
const WRITER_WAIT_MASK: u32 = ((1 << 31) -1) & !READER_MASK;



// 等待写者数量加 1 的辅助函数（数值）
const fn writer_wait_add_num() -> u32{
    1 << 16
}


pub struct RwLock<T> {
    state: AtomicU32,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send+Sync> Sync for RwLock<T> {}


impl<T: Send> RwLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            data: UnsafeCell::new(data),
        }
    }

    pub fn read(&self) -> ReadGuard<'_, T> {
        loop {
            
            let state = self.state.load(Ordering::Acquire);

            // 当前有写者持有锁，或者是有写者正在等，就自旋
            if state & (WRITER_WAIT_MASK | WRITER_HOLDING) != 0{
                std::hint::spin_loop();
                continue;
            }

            // 读者到达最大容量了
            // 防止溢出, 直接 panic, 而不是静默绕回
            if state & READER_MASK == READER_MASK {
                panic!("too many readers");
            }

            // 没有写者持有锁，也没有写者在等，读者的数目也没到最大值
            // 此时加入新的读者
            // 这是可以直接加 1 的，因为是在 低位
            // 成功直接返回，否则持续自旋
            if self
                .state
                .compare_exchange_weak(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return ReadGuard { lock: self };
            }
            std::hint::spin_loop();
        }
    }



    pub fn write(&self) -> WriteGuard<'_, T> {

        // 第一阶段：注册为等待写者（只执行一次）
        loop {

            let state = self.state.load(Ordering::Relaxed);

            // 等待写者的数量已经到达上限了
            if state & WRITER_WAIT_MASK == WRITER_WAIT_MASK{
                panic!("too many writer waiting")
            }
            
            if self.state.compare_exchange_weak(state,state + writer_wait_add_num(),Ordering::AcqRel,Ordering::Relaxed,
            ).is_ok() {
                break;
            }

            std::hint::spin_loop();
        }
        

        // 第二阶段：竞争写锁
        loop{

            // 这里必须是 Acquire 从而保证之前的操作，不会放在后面！
            let state = self.state.load(Ordering::Acquire);

            // 还有读者持有, 等它们读完
            if state & READER_MASK != 0 {
                std::hint::spin_loop();
                continue;
            }
            // 有写者正在写
            if state & WRITER_HOLDING != 0 {
                std::hint::spin_loop();
                continue;
            }

            /* 这样相当于在等待的读者当中是随机竞争的 */
            // 这里是关键，此时 没有 HOLD，没有读者，但是可能有正在等待的写者
            // 解决办法就是 通过 CAS 让其竞争！这样起码有一个写者可以拿到 写锁
            // 直接将等待写者的数目减 1 ，然后 HOLD 位 置 1
            // 这样的话语，只要 CAS 推进成功了，就是 HOLD 为 1 ，并且 等待写 的数量也减去 1 
            let new = (state - writer_wait_add_num()) | WRITER_HOLDING;

            if self
                .state
                .compare_exchange_weak(
                    state, 
                    new, 
                    Ordering::Acquire, 
                    Ordering::Relaxed)
                .is_ok(){
                return WriteGuard { lock: self };
            }
            std::hint::spin_loop();
        }
    }
}



pub struct ReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}
impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}
impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

pub struct WriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}
impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}
impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}
impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock
            .state
            .fetch_and(!WRITER_HOLDING , Ordering::Release); // 这里将 HOLD 清 0 即可
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_multiple_readers() {
        let lock = Arc::new(RwLock::new(0u32));
        let mut handles = vec![];
        for _ in 0..10 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                let g = l.read();
                assert_eq!(*g, 0);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_writer_excludes_readers() {
        let lock = Arc::new(RwLock::new(0u32));
        let lock_w = Arc::clone(&lock);
        let writer = thread::spawn(move || {
            let mut g = lock_w.write();
            *g = 42;
        });
        writer.join().unwrap();
        let g = lock.read();
        assert_eq!(*g, 42);
    }

    #[test]
    fn test_concurrent_reads_after_write() {
        let lock = Arc::new(RwLock::new(Vec::<i32>::new()));
        {
            let mut g = lock.write();
            g.push(1);
            g.push(2);
        }
        let mut handles = vec![];
        for _ in 0..5 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                let g = l.read();
                assert_eq!(g.len(), 2);
                assert_eq!(&*g, &[1, 2]);
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_concurrent_writes_serialized() {
        let lock = Arc::new(RwLock::new(0u64));
        let mut handles = vec![];
        for _ in 0..10 {
            let l = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let mut g = l.write();
                    *g += 1;
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(*lock.read(), 1000);
    }
}


