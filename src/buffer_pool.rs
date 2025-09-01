//! High-performance buffer pool for network operations
//! 
//! This module provides a thread-safe buffer pool that minimizes allocations
//! during high-frequency network operations. Buffers are reused to reduce
//! garbage collection pressure and improve cache locality.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// A thread-safe buffer pool for network I/O operations
/// 
/// The buffer pool maintains a collection of pre-allocated byte vectors
/// that can be reused across multiple network operations to minimize
/// allocation overhead.
/// 
/// # Examples
/// 
/// ```rust
/// use horizon_sockets::buffer_pool::BufferPool;
/// 
/// let pool = BufferPool::new(64, 2048); // 64 buffers of 2KB each
/// let mut buffer = pool.acquire();
/// 
/// // Use buffer for network operation
/// buffer.resize(1500, 0);
/// 
/// // Return buffer to pool when done
/// pool.release(buffer);
/// ```
#[derive(Clone, Debug)]
pub struct BufferPool {
    /// Internal storage for available buffers
    buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,
    /// Default capacity for new buffers
    default_capacity: usize,
    /// Maximum number of buffers to keep in pool
    max_buffers: usize,
}

impl BufferPool {
    /// Creates a new buffer pool with the specified parameters
    /// 
    /// # Arguments
    /// 
    /// * `initial_count` - Number of buffers to pre-allocate
    /// * `buffer_capacity` - Default capacity for each buffer in bytes
    /// 
    /// # Returns
    /// 
    /// A new `BufferPool` instance ready for use
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// // Create pool with 32 buffers of 1KB each
    /// let pool = BufferPool::new(32, 1024);
    /// ```
    pub fn new(initial_count: usize, buffer_capacity: usize) -> Self {
        let mut buffers = VecDeque::with_capacity(initial_count * 2);
        
        // Pre-allocate initial buffers
        for _ in 0..initial_count {
            buffers.push_back(Vec::with_capacity(buffer_capacity));
        }
        
        Self {
            buffers: Arc::new(Mutex::new(buffers)),
            default_capacity: buffer_capacity,
            max_buffers: initial_count * 2, // Allow pool to grow up to 2x initial size
        }
    }
    
    /// Acquires a buffer from the pool
    /// 
    /// If no buffers are available in the pool, a new buffer is allocated
    /// with the default capacity. This ensures the operation never blocks.
    /// 
    /// # Returns
    /// 
    /// A `Vec<u8>` buffer ready for use
    /// 
    /// # Performance Notes
    /// 
    /// - O(1) operation when buffers are available
    /// - Falls back to allocation if pool is empty
    /// - Buffer contents are not cleared for performance
    pub fn acquire(&self) -> Vec<u8> {
        let mut buffers = self.buffers.lock().unwrap();
        
        buffers.pop_front().unwrap_or_else(|| {
            // Pool is empty, allocate new buffer
            Vec::with_capacity(self.default_capacity)
        })
    }
    
    /// Returns a buffer to the pool for reuse
    /// 
    /// The buffer is cleared and returned to the pool for future use.
    /// If the pool is at capacity, the buffer is dropped to prevent
    /// unbounded memory growth.
    /// 
    /// # Arguments
    /// 
    /// * `buffer` - The buffer to return to the pool
    /// 
    /// # Performance Notes
    /// 
    /// - Buffer is cleared but capacity is preserved
    /// - O(1) operation
    /// - Excess buffers are dropped to limit memory usage
    pub fn release(&self, mut buffer: Vec<u8>) {
        let mut buffers = self.buffers.lock().unwrap();
        
        if buffers.len() < self.max_buffers {
            // Clear buffer contents but preserve capacity
            buffer.clear();
            buffers.push_back(buffer);
        }
        // If pool is full, buffer is dropped automatically
    }
    
    /// Returns the number of buffers currently available in the pool
    /// 
    /// This is useful for monitoring pool utilization and performance tuning.
    /// 
    /// # Returns
    /// 
    /// The number of available buffers in the pool
    pub fn available_count(&self) -> usize {
        self.buffers.lock().unwrap().len()
    }
    
    /// Returns the default buffer capacity in bytes
    /// 
    /// # Returns
    /// 
    /// The default capacity for buffers created by this pool
    pub fn default_capacity(&self) -> usize {
        self.default_capacity
    }
    
    /// Acquires multiple buffers from the pool efficiently
    /// 
    /// This is optimized for batch operations where multiple buffers
    /// are needed simultaneously, such as UDP batch receive operations.
    /// 
    /// # Arguments
    /// 
    /// * `count` - Number of buffers to acquire
    /// 
    /// # Returns
    /// 
    /// A vector containing the requested number of buffers
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// let pool = BufferPool::new(64, 2048);
    /// let buffers = pool.acquire_batch(16);
    /// 
    /// // Use buffers for batch network operation
    /// // ...
    /// 
    /// pool.release_batch(buffers);
    /// ```
    pub fn acquire_batch(&self, count: usize) -> Vec<Vec<u8>> {
        let mut buffers = self.buffers.lock().unwrap();
        let mut result = Vec::with_capacity(count);
        
        // First, try to fulfill from pool
        let available = buffers.len().min(count);
        for _ in 0..available {
            if let Some(buffer) = buffers.pop_front() {
                result.push(buffer);
            }
        }
        
        // Allocate remaining buffers if needed
        for _ in available..count {
            result.push(Vec::with_capacity(self.default_capacity));
        }
        
        result
    }
    
    /// Returns multiple buffers to the pool efficiently
    /// 
    /// This is the counterpart to `acquire_batch` for returning
    /// multiple buffers at once.
    /// 
    /// # Arguments
    /// 
    /// * `batch` - Vector of buffers to return to the pool
    pub fn release_batch(&self, batch: Vec<Vec<u8>>) {
        let mut buffers = self.buffers.lock().unwrap();
        
        for mut buffer in batch {
            if buffers.len() < self.max_buffers {
                buffer.clear();
                buffers.push_back(buffer);
            }
            // Excess buffers are dropped
        }
    }
}

impl Default for BufferPool {
    /// Creates a default buffer pool optimized for typical network workloads
    /// 
    /// Default configuration:
    /// - 64 buffers initially allocated  
    /// - 2048 bytes per buffer (typical MTU size)
    /// - Pool can grow to 128 buffers maximum
    fn default() -> Self {
        Self::new(64, 2048)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_buffer_pool_basic_operations() {
        let pool = BufferPool::new(4, 1024);
        
        // Pool should start with 4 available buffers
        assert_eq!(pool.available_count(), 4);
        
        // Acquire a buffer
        let buffer = pool.acquire();
        assert_eq!(buffer.capacity(), 1024);
        assert_eq!(pool.available_count(), 3);
        
        // Return the buffer
        pool.release(buffer);
        assert_eq!(pool.available_count(), 4);
    }
    
    #[test]
    fn test_buffer_pool_batch_operations() {
        let pool = BufferPool::new(8, 512);
        
        // Acquire batch of buffers
        let buffers = pool.acquire_batch(6);
        assert_eq!(buffers.len(), 6);
        assert_eq!(pool.available_count(), 2);
        
        // Return batch
        pool.release_batch(buffers);
        assert_eq!(pool.available_count(), 8);
    }
    
    #[test]
    fn test_buffer_pool_overflow_allocation() {
        let pool = BufferPool::new(2, 256);
        
        // Acquire more buffers than available
        let buffers = pool.acquire_batch(5);
        assert_eq!(buffers.len(), 5);
        assert_eq!(pool.available_count(), 0);
        
        // All buffers should have correct capacity
        for buffer in buffers {
            assert_eq!(buffer.capacity(), 256);
        }
    }
}