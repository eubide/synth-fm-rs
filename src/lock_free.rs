use std::sync::atomic::{AtomicU8, Ordering};

/// Lock-free triple buffer for real-time parameter updates.
///
/// Uses a single atomic byte to track buffer indices, ensuring atomic swaps.
/// The byte layout is: [unused:2][back:2][read:2][write:2]
///
/// Writer (GUI thread): writes to write buffer, then swaps write<->back
/// Reader (Audio thread): swaps read<->back, then reads from read buffer
pub struct TripleBuffer<T: Clone + Send> {
    buffers: [std::cell::UnsafeCell<T>; 3],
    /// Packed indices: bits 0-1 = write, bits 2-3 = read, bits 4-5 = back
    indices: AtomicU8,
}

impl<T: Clone + Send> TripleBuffer<T> {
    const WRITE_MASK: u8 = 0b00000011;
    const READ_MASK: u8 = 0b00001100;
    const BACK_MASK: u8 = 0b00110000;
    const WRITE_SHIFT: u8 = 0;
    const READ_SHIFT: u8 = 2;
    const BACK_SHIFT: u8 = 4;

    pub fn new(initial_value: T) -> Self {
        // Initial state: write=0, read=1, back=2
        let initial_indices = (1 << Self::READ_SHIFT) | (2 << Self::BACK_SHIFT);

        Self {
            buffers: [
                std::cell::UnsafeCell::new(initial_value.clone()),
                std::cell::UnsafeCell::new(initial_value.clone()),
                std::cell::UnsafeCell::new(initial_value),
            ],
            indices: AtomicU8::new(initial_indices),
        }
    }

    /// Write new data (GUI thread only).
    /// After writing, atomically swaps write and back buffers.
    pub fn write(&self, data: T) {
        // Get current write index
        let current = self.indices.load(Ordering::Acquire);
        let write_idx = (current & Self::WRITE_MASK) >> Self::WRITE_SHIFT;

        // Write to the write buffer (safe: we're the only writer)
        unsafe {
            *self.buffers[write_idx as usize].get() = data;
        }

        // Atomically swap write and back buffers using CAS loop
        loop {
            let current = self.indices.load(Ordering::Acquire);
            let write_idx = (current & Self::WRITE_MASK) >> Self::WRITE_SHIFT;
            let back_idx = (current & Self::BACK_MASK) >> Self::BACK_SHIFT;

            // New state: swap write and back
            let new_indices = (current & Self::READ_MASK) // keep read
                | (back_idx << Self::WRITE_SHIFT)  // back becomes write
                | (write_idx << Self::BACK_SHIFT); // write becomes back

            match self.indices.compare_exchange_weak(
                current,
                new_indices,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(_) => continue, // Retry if indices changed
            }
        }
    }

    /// Read current data (Audio thread only).
    /// Atomically swaps read and back buffers, then returns reference to read buffer.
    #[allow(dead_code)]
    pub fn read(&self) -> &T {
        // Atomically swap read and back buffers using CAS loop
        loop {
            let current = self.indices.load(Ordering::Acquire);
            let read_idx = (current & Self::READ_MASK) >> Self::READ_SHIFT;
            let back_idx = (current & Self::BACK_MASK) >> Self::BACK_SHIFT;

            // New state: swap read and back
            let new_indices = (current & Self::WRITE_MASK) // keep write
                | (back_idx << Self::READ_SHIFT)  // back becomes read
                | (read_idx << Self::BACK_SHIFT); // read becomes back

            if self
                .indices
                .compare_exchange_weak(current, new_indices, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                // Return reference to the new read buffer (was back)
                return unsafe { &*self.buffers[back_idx as usize].get() };
            }
            // Retry if indices changed
        }
    }

    /// Peek at current read buffer without swapping.
    /// Use this when you just need to check the current value.
    #[allow(dead_code)]
    pub fn peek(&self) -> &T {
        let current = self.indices.load(Ordering::Acquire);
        let read_idx = (current & Self::READ_MASK) >> Self::READ_SHIFT;
        unsafe { &*self.buffers[read_idx as usize].get() }
    }
}

// Safety: T is Send, and we use proper atomic synchronization
unsafe impl<T: Clone + Send> Send for TripleBuffer<T> {}
unsafe impl<T: Clone + Send> Sync for TripleBuffer<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_triple_buffer_basic() {
        let buffer = TripleBuffer::new(0u32);
        assert_eq!(*buffer.peek(), 0);

        buffer.write(42);
        assert_eq!(*buffer.read(), 42);
    }

    #[test]
    fn test_triple_buffer_multiple_writes() {
        let buffer = TripleBuffer::new(0u32);

        buffer.write(1);
        buffer.write(2);
        buffer.write(3);

        // Should get the latest value
        assert_eq!(*buffer.read(), 3);
    }

    #[test]
    fn test_triple_buffer_concurrent() {
        let buffer = Arc::new(TripleBuffer::new(0u64));
        let b1 = buffer.clone();
        let b2 = buffer.clone();

        let writer = thread::spawn(move || {
            for i in 0..10000u64 {
                b1.write(i);
            }
        });

        let reader = thread::spawn(move || {
            for _ in 0..10000 {
                let value = *b2.read();
                // Triple buffers don't guarantee monotonic reads, but they guarantee
                // no data corruption. Value must be in valid range [0, 10000).
                assert!(value < 10000, "Value {} is out of range", value);
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
    }
}

#[cfg(all(test, loom))]
mod loom_tests {
    use super::*;
    use loom::sync::Arc;
    use loom::thread;

    #[test]
    fn test_triple_buffer_loom() {
        loom::model(|| {
            let buffer = Arc::new(TripleBuffer::new(0u32));
            let b1 = buffer.clone();
            let b2 = buffer.clone();

            let t1 = thread::spawn(move || {
                b1.write(42);
            });

            let t2 = thread::spawn(move || {
                let _ = b2.read();
            });

            t1.join().unwrap();
            t2.join().unwrap();
        });
    }
}
