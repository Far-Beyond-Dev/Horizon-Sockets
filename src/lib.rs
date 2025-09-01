//! # Horizon Sockets - High Performance Network Library
//!
//! Horizon Sockets is a high-performance, low-latency networking library for Rust
//! designed for applications that require ultra-fast socket operations with minimal
//! overhead. The library provides both UDP and TCP implementations with extensive
//! performance tuning options.
//!
//! ## Key Features
//!
//! - **Multiple Runtime Backends**: Choose between `mio` (epoll/kqueue/IOCP) or `monoio` (io_uring/IOCP)
//! - **Cross-Platform Support**: Full support for Linux, Windows, macOS, and other Unix systems
//! - **Low-Latency Optimizations**: Built-in support for TCP_NODELAY, SO_BUSY_POLL, and other performance features
//! - **Batch Operations**: High-performance batch UDP operations using `recvmmsg` on Linux
//! - **Buffer Pool Management**: Efficient memory management with reusable buffer pools
//! - **CPU Affinity Control**: Thread pinning utilities for consistent performance
//! - **Comprehensive Configuration**: Extensive tuning through `NetConfig`
//!
//! ## Quick Example
//!
//! ```rust,no_run
//! use horizon_sockets::{NetConfig, udp::Udp, buffer_pool::BufferPool};
//! use std::net::SocketAddr;
//!
//! fn main() -> std::io::Result<()> {
//!     // Configure for low latency
//!     let config = NetConfig {
//!         busy_poll: Some(50), // 50 microseconds busy polling
//!         recv_buf: Some(4 << 20), // 4MB receive buffer
//!         ..Default::default()
//!     };
//!
//!     // Create socket with optimized configuration
//!     let socket = Udp::bind("0.0.0.0:8080".parse()?, &config)?;
//!
//!     // Use buffer pool for efficient memory management
//!     let pool = BufferPool::new(64, 2048);
//!     let mut buffers = pool.acquire_batch(32);
//!     let mut addrs = vec![SocketAddr::from(([0,0,0,0], 0)); 32];
//!
//!     loop {
//!         match socket.recv_batch(&mut buffers, &mut addrs) {
//!             Ok(count) => {
//!                 // Process received packets
//!                 for i in 0..count {
//!                     // Echo back received data
//!                     socket.send_to(&buffers[i], addrs[i])?;
//!                 }
//!             }
//!             Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
//!             Err(e) => return Err(e),
//!         }
//!     }
//! }
//! ```
//!
//! ## Architecture
//!
//! The library is organized into several key modules:
//!
//! - [`config`]: Network configuration and performance tuning parameters
//! - [`raw`]: Low-level socket operations and platform-specific implementations
//! - [`udp`]: High-level UDP socket interface with batch operations
//! - [`tcp`]: High-level TCP socket interface with connection management
//! - [`buffer_pool`]: Memory-efficient buffer pool for network operations
//! - [`affinity`]: CPU affinity and thread pinning utilities
//! - [`rt`]: Runtime backends (mio/monoio) for async I/O operations
//!
//! ## Performance Tips
//!
//! 1. **Use Buffer Pools**: Always use `BufferPool` for high-frequency operations
//! 2. **Pin Threads**: Use `affinity::pin_to_cpu()` for consistent latency
//! 3. **Tune Buffer Sizes**: Start with 1-4MB buffers, adjust based on workload
//! 4. **Enable Busy Polling**: Use `busy_poll` on dedicated cores for ultra-low latency
//! 5. **Batch Operations**: Use `recv_batch` for UDP to minimize syscall overhead

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

/// CPU affinity and thread pinning utilities
pub mod affinity;
/// Memory-efficient buffer pool for network operations
pub mod buffer_pool;
/// Network configuration and performance tuning
pub mod config;
/// Low-level socket operations and platform abstractions  
pub mod raw;
/// High-performance TCP socket implementation
pub mod tcp;
/// High-performance UDP socket implementation
pub mod udp;

cfg_if::cfg_if! {
    if #[cfg(feature = "monoio-runtime")] {
        /// Runtime implementation using monoio (io_uring on Linux, IOCP on Windows)
        pub mod rt { pub use crate::rt_monoio::*; }
        mod rt_monoio;
    } else if #[cfg(feature = "mio-runtime")] {
        /// Runtime implementation using mio (epoll/kqueue/IOCP)
        pub mod rt { pub use crate::rt_mio::*; }
        mod rt_mio;
    } else {
        compile_error!("Enable one of: mio-runtime (default) or monoio-runtime");
    }
}

pub use buffer_pool::BufferPool;
/// Convenience re-exports for common types and functions
///
/// These re-exports provide easy access to the most commonly used
/// types and functions without requiring full module paths.
pub use config::{NetConfig, apply_low_latency};
pub use rt::{NetHandle, Runtime};

// Re-export main socket types for easier access
pub use tcp::{TcpListener, TcpStream};
pub use udp::Udp;

// Re-export affinity utilities for performance tuning
pub use affinity::{get_cpu_count, get_numa_topology, pin_to_cpu, pin_to_cpus};
