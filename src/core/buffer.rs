//! Zero-copy buffer management for high-performance packet processing
//! 
//! This module provides a lock-free buffer pool system that minimizes
//! memory allocations during packet injection operations.

use bytes::{Bytes, BytesMut};
use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use tracing::{debug, warn};

/// High-performance buffer pool for packet processing
pub struct PacketBuffer {
    pool: Arc<ArrayQueue<BytesMut>>,
    buffer_size: usize,
    pool_size: usize,
}

impl PacketBuffer {
    /// Create a new buffer pool with specified parameters
    pub fn new(pool_size: usize, buffer_size: usize) -> Self {
        let pool = Arc::new(ArrayQueue::new(pool_size));
        
        // Pre-populate the pool with buffers
        for _ in 0..pool_size {
            let _ = pool.push(BytesMut::with_capacity(buffer_size));
        }
        
        debug!("Created buffer pool with {} buffers of {} bytes each", pool_size, buffer_size);
        
        Self {
            pool,
            buffer_size,
            pool_size,
        }
    }
    
    /// Acquire a buffer from the pool (non-blocking)
    #[inline]
    pub fn acquire(&self) -> Option<BytesMut> {
        match self.pool.pop() {
            Some(mut buffer) => {
                buffer.clear();
                debug!("Acquired buffer from pool");
                Some(buffer)
            }
            None => {
                warn!("Buffer pool exhausted, creating new buffer");
                Some(BytesMut::with_capacity(self.buffer_size))
            }
        }
    }
    
    /// Release a buffer back to the pool
    #[inline]
    pub fn release(&self, mut buffer: BytesMut) {
        // Only return buffers that match our expected size
        if buffer.capacity() == self.buffer_size {
            buffer.clear();
            if self.pool.push(buffer).is_err() {
                // Pool is full, drop the buffer
                debug!("Buffer pool full, dropping buffer");
            }
        }
    }
    
    /// Get current pool statistics
    pub fn stats(&self) -> BufferStats {
        BufferStats {
            available: self.pool.len(),
            total: self.pool_size,
            buffer_size: self.buffer_size,
        }
    }
}

/// Buffer pool statistics
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub available: usize,
    pub total: usize,
    pub buffer_size: usize,
}

impl BufferStats {
    pub fn utilization(&self) -> f64 {
        (self.total - self.available) as f64 / self.total as f64
    }
}

/// Thread-local buffer cache for even better performance
pub struct ThreadLocalBuffer {
    local_buffer: Option<BytesMut>,
    pool: Arc<PacketBuffer>,
}

impl ThreadLocalBuffer {
    pub fn new(pool: Arc<PacketBuffer>) -> Self {
        Self {
            local_buffer: None,
            pool,
        }
    }
    
    /// Get a buffer, preferring the thread-local cache
    pub fn get(&mut self) -> BytesMut {
        if let Some(buffer) = self.local_buffer.take() {
            debug!("Using thread-local buffer");
            buffer
        } else {
            self.pool.acquire().unwrap_or_else(|| BytesMut::with_capacity(2048))
        }
    }
    
    /// Return a buffer to the thread-local cache or pool
    pub fn put(&mut self, buffer: BytesMut) {
        if self.local_buffer.is_none() {
            self.local_buffer = Some(buffer);
        } else {
            self.pool.release(buffer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_buffer_pool_basic() {
        let pool = PacketBuffer::new(10, 2048);
        
        // Acquire and release buffers
        let buffer1 = pool.acquire().expect("Should get buffer");
        assert_eq!(buffer1.capacity(), 2048);
        
        pool.release(buffer1);
        
        let stats = pool.stats();
        assert_eq!(stats.available, 10);
        assert_eq!(stats.total, 10);
    }
    
    #[test]
    fn test_buffer_pool_exhaustion() {
        let pool = PacketBuffer::new(2, 1024);
        
        // Exhaust the pool
        let _b1 = pool.acquire().expect("Should get buffer");
        let _b2 = pool.acquire().expect("Should get buffer");
        
        // Should still get a buffer (newly allocated)
        let _b3 = pool.acquire().expect("Should get buffer");
    }
    
    #[test]
    fn test_thread_local_buffer() {
        let pool = Arc::new(PacketBuffer::new(5, 1024));
        let mut tl_buffer = ThreadLocalBuffer::new(pool.clone());
        
        let buffer = tl_buffer.get();
        tl_buffer.put(buffer);
        
        // Should reuse the thread-local buffer
        let _buffer2 = tl_buffer.get();
    }
}