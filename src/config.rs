//! Network configuration and performance tuning
//!
//! This module provides comprehensive configuration options for optimizing
//! network performance across different platforms and workloads. The `NetConfig`
//! struct allows fine-tuning of socket parameters for latency, throughput,
//! and specific application requirements.
//!
//! # Performance Categories
//!
//! ## Latency Optimization
//! - `tcp_nodelay`: Disables Nagle's algorithm for immediate send
//! - `tcp_quickack`: Reduces ACK delay on Linux
//! - `busy_poll`: Polls network device for specified microseconds
//!
//! ## Throughput Optimization  
//! - `recv_buf`/`send_buf`: Larger socket buffers for high-bandwidth applications
//! - `reuse_port`: Enables SO_REUSEPORT for load balancing across threads
//!
//! ## Quality of Service
//! - `tos`: DSCP/TOS marking for traffic prioritization
//! - `hop_limit`: IPv6 hop limit control
//!
//! # Examples
//!
//! ```rust
//! use horizon_sockets::NetConfig;
//!
//! // Ultra-low latency configuration
//! let low_latency = NetConfig {
//!     tcp_nodelay: true,
//!     tcp_quickack: true,
//!     busy_poll: Some(50), // 50μs busy polling
//!     recv_buf: Some(256 * 1024), // Smaller buffers for lower latency
//!     send_buf: Some(256 * 1024),
//!     ..Default::default()
//! };
//!
//! // High-throughput configuration
//! let high_throughput = NetConfig {
//!     recv_buf: Some(16 << 20), // 16MB buffers
//!     send_buf: Some(16 << 20),
//!     reuse_port: true,
//!     busy_poll: None, // Disable busy polling for shared systems
//!     ..Default::default()
//! };
//! ```

use std::io;
use crate::raw;
#[cfg(target_os = "linux")] use std::time::Duration;

/// Network configuration for performance tuning and optimization
/// 
/// This structure contains all configurable parameters for optimizing
/// network socket performance. Different combinations of settings are
/// suitable for different workloads:
/// 
/// - **Low Latency**: Small buffers, busy polling, TCP_NODELAY
/// - **High Throughput**: Large buffers, SO_REUSEPORT, no busy polling  
/// - **Mixed Workload**: Balanced settings with moderate buffer sizes
/// 
/// All parameters are optional and use sensible defaults when not specified.
/// Platform-specific options are ignored on unsupported platforms.
#[derive(Clone, Debug, PartialEq)]
pub struct NetConfig {
/// Enable TCP_NODELAY to disable Nagle's algorithm
/// 
/// When `true`, TCP packets are sent immediately rather than being
/// buffered for efficiency. Essential for low-latency applications.
/// Ignored for UDP sockets.
/// 
/// **Default**: `true`
pub tcp_nodelay: bool,

/// Enable TCP_QUICKACK for faster ACK responses (Linux only)
/// 
/// Reduces the delay before sending ACKs, which can improve
/// request-response latency. Only effective on Linux systems.
/// Ignored on other platforms and for UDP sockets.
/// 
/// **Default**: `true`
pub tcp_quickack: bool,

/// Enable SO_REUSEPORT for load balancing (Linux/BSD only)
/// 
/// Allows multiple sockets to bind to the same port for load
/// distribution across threads/processes. Requires kernel support.
/// 
/// **Default**: `true`
pub reuse_port: bool,

/// SO_BUSY_POLL timeout in microseconds (Linux only)
/// 
/// Enables busy polling on the network device for the specified
/// duration before falling back to interrupt-driven I/O. Reduces
/// latency at the cost of CPU usage. Recommended range: 10-100μs.
/// 
/// - `None`: Disabled (default for shared systems)
/// - `Some(μs)`: Busy poll for specified microseconds
/// 
/// **Default**: `None`
pub busy_poll: Option<u32>,

/// Socket receive buffer size in bytes
/// 
/// Larger buffers can improve throughput but may increase latency.
/// The kernel may adjust the actual size based on system limits.
/// 
/// - Low latency: 64KB - 512KB
/// - Balanced: 1MB - 4MB  
/// - High throughput: 8MB - 64MB
/// 
/// **Default**: `Some(4MB)`
pub recv_buf: Option<usize>,

/// Socket send buffer size in bytes
/// 
/// Larger buffers allow more data to be buffered for sending,
/// which can improve throughput for bulk transfers.
/// 
/// **Default**: `Some(4MB)`
pub send_buf: Option<usize>,

/// IP Type of Service / DSCP marking
/// 
/// Sets the TOS byte in IP headers for traffic classification
/// and QoS. Common values:
/// 
/// - `0x10`: Low delay
/// - `0x08`: High throughput  
/// - `0x04`: High reliability
/// - `0x02`: Low cost
/// 
/// **Default**: `None` (no marking)
pub tos: Option<u32>,

/// IPv6-only socket configuration
/// 
/// Controls whether IPv6 sockets accept IPv4 connections:
/// 
/// - `Some(true)`: IPv6 only, reject IPv4
/// - `Some(false)`: Dual-stack, accept both IPv4 and IPv6
/// - `None`: Use system default
/// 
/// **Default**: `Some(false)` (dual-stack)
pub ipv6_only: Option<bool>,

/// IPv6 hop limit (TTL equivalent)
/// 
/// Maximum number of hops for IPv6 packets. Equivalent to
/// IPv4 TTL. System default is typically 64.
/// 
/// **Default**: `None` (system default)
pub hop_limit: Option<i32>,

/// TCP listen backlog size
/// 
/// Maximum number of pending connections in the accept queue.
/// Larger values can handle connection bursts but use more memory.
/// 
/// **Default**: `Some(1024)`
pub tcp_backlog: Option<i32>,

/// Event loop polling timeout in milliseconds
/// 
/// Maximum time to wait for events before returning from poll.
/// Shorter timeouts provide better responsiveness but higher CPU usage.
/// 
/// - Low latency: 1-10ms
/// - Balanced: 10-50ms
/// - Power efficient: 100ms+
/// 
/// **Default**: `Some(10)`
pub poll_timeout_ms: Option<u64>,
}


impl Default for NetConfig {
    /// Creates a default configuration optimized for balanced performance
    /// 
    /// The default settings provide a good balance between latency and
    /// throughput, suitable for most applications:
    /// 
    /// - TCP optimizations enabled (NODELAY, QUICKACK)
    /// - 4MB socket buffers for good throughput
    /// - SO_REUSEPORT enabled for scalability
    /// - Dual-stack IPv6 support
    /// - Conservative polling timeout
    /// - No busy polling (suitable for shared systems)
    fn default() -> Self {
        Self {
            tcp_nodelay: true,
            tcp_quickack: true,
            reuse_port: true,
            busy_poll: None,
            recv_buf: Some(4 << 20), // 4 MiB - increased from 1MB
            send_buf: Some(4 << 20),  // 4 MiB - increased from 1MB
            tos: None,
            ipv6_only: Some(false), // Dual-stack by default
            hop_limit: None,
            tcp_backlog: Some(1024),
            poll_timeout_ms: Some(10),
        }
    }
}


impl NetConfig {
    /// Creates a configuration optimized for ultra-low latency
    /// 
    /// This preset is designed for latency-sensitive applications like
    /// high-frequency trading, gaming, or real-time communication.
    /// 
    /// # Features
    /// - Small socket buffers (256KB) to minimize queuing delay
    /// - Busy polling enabled (50μs) for immediate packet processing
    /// - All TCP latency optimizations enabled
    /// - Aggressive polling timeout (1ms)
    /// 
    /// # Trade-offs
    /// - Higher CPU usage due to busy polling
    /// - Lower maximum throughput due to small buffers
    /// - May not be suitable for shared/virtualized environments
    pub fn low_latency() -> Self {
        Self {
            tcp_nodelay: true,
            tcp_quickack: true,
            reuse_port: true,
            busy_poll: Some(50), // 50μs busy polling
            recv_buf: Some(256 * 1024), // 256KB buffers
            send_buf: Some(256 * 1024),
            tos: Some(0x10), // Low delay DSCP marking
            ipv6_only: Some(false),
            hop_limit: None,
            tcp_backlog: Some(512), // Smaller backlog for faster processing
            poll_timeout_ms: Some(1), // 1ms timeout for responsiveness
        }
    }
    
    /// Creates a configuration optimized for high throughput
    /// 
    /// This preset maximizes data transfer rates for bulk operations,
    /// streaming, or file transfers.
    /// 
    /// # Features
    /// - Large socket buffers (16MB) for maximum throughput
    /// - No busy polling to conserve CPU for data processing
    /// - SO_REUSEPORT for multi-threaded scaling
    /// - Longer polling timeout for efficiency
    /// 
    /// # Trade-offs
    /// - Higher memory usage due to large buffers
    /// - Potentially higher latency due to buffering
    /// - Optimized for sustained transfers, not request-response
    pub fn high_throughput() -> Self {
        Self {
            tcp_nodelay: false, // Allow Nagle for efficiency
            tcp_quickack: false, // Delayed ACKs for efficiency
            reuse_port: true,
            busy_poll: None, // No busy polling
            recv_buf: Some(16 << 20), // 16MB buffers
            send_buf: Some(16 << 20),
            tos: Some(0x08), // High throughput DSCP marking
            ipv6_only: Some(false),
            hop_limit: None,
            tcp_backlog: Some(2048), // Large backlog for connection bursts
            poll_timeout_ms: Some(50), // Longer timeout for efficiency
        }
    }
    
    /// Creates a configuration for power-efficient operation
    /// 
    /// This preset minimizes CPU usage and power consumption,
    /// suitable for battery-powered devices or background services.
    /// 
    /// # Features
    /// - Moderate buffer sizes for balance
    /// - No busy polling to save CPU cycles
    /// - Longer polling timeouts to reduce wakeups
    /// - Conservative settings throughout
    pub fn power_efficient() -> Self {
        Self {
            tcp_nodelay: true,
            tcp_quickack: false, // Reduce CPU overhead
            reuse_port: false, // Simpler socket management
            busy_poll: None,
            recv_buf: Some(512 * 1024), // 512KB buffers
            send_buf: Some(512 * 1024),
            tos: None,
            ipv6_only: Some(false),
            hop_limit: None,
            tcp_backlog: Some(256),
            poll_timeout_ms: Some(100), // Long timeout to reduce wakeups
        }
    }
}

/// Applies network optimizations to a raw socket
/// 
/// This function takes a platform-specific raw socket handle and applies
/// all the optimizations specified in the `NetConfig`. It must be called
/// before the socket is converted to a standard library type.
/// 
/// # Arguments
/// 
/// * `os` - Platform-specific raw socket handle
/// * `domain` - IP protocol family (IPv4 or IPv6) 
/// * `ty` - Socket type (TCP stream or UDP datagram)
/// * `cfg` - Configuration with optimization parameters
/// 
/// # Returns
/// 
/// `Ok(())` on success, or an `io::Error` if any optimization fails
/// 
/// # Platform Support
/// 
/// - **Linux**: Full support for all optimizations including SO_BUSY_POLL
/// - **Windows**: Most optimizations supported via WinSock APIs
/// - **macOS/BSD**: Standard socket options supported
/// - **Other Unix**: Basic socket options only
/// 
/// Unsupported options are silently ignored rather than causing errors.
/// 
/// # Safety
/// 
/// This function performs low-level socket operations using platform-specific
/// APIs. The caller must ensure the socket handle is valid and matches the
/// specified domain and type.
pub fn apply_low_latency(os: raw::OsSocket, domain: raw::Domain, ty: raw::Type, cfg: &NetConfig) -> io::Result<()> {
    use crate::raw as r;

    // Configure socket buffer sizes for optimal performance
    if let Some(sz) = cfg.recv_buf { r::set_recv_buffer(os, sz as i32)?; }
    if let Some(sz) = cfg.send_buf { r::set_send_buffer(os, sz as i32)?; }

    // Apply Quality of Service / DSCP marking
    if let Some(tos) = cfg.tos {
        match domain { 
            r::Domain::Ipv4 => r::set_tos_v4(os, tos as i32)?, 
            r::Domain::Ipv6 => r::set_tos_v6(os, tos as i32)?, 
        }
    }

    // Configure IPv6-specific options
    if let r::Domain::Ipv6 = domain {
        if let Some(only) = cfg.ipv6_only { r::set_ipv6_only(os, only)?; }
        if let Some(hops) = cfg.hop_limit { r::set_ipv6_hop_limit(os, hops)?; }
    }

    // Apply Linux-specific performance optimizations
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        if cfg.reuse_port { r::set_reuse_port(os, true)?; }
        if let Some(us) = cfg.busy_poll { 
            // Busy polling: poll network device for specified microseconds
            let _ = r::set_busy_poll(os, us); 
        }
        if cfg.tcp_quickack && ty == r::Type::Stream { 
            // TCP Quick ACK: send ACKs immediately rather than delaying
            let _ = r::set_tcp_quickack(os, true); 
        }
    }

    // Apply TCP-specific optimizations
    if ty == r::Type::Stream && cfg.tcp_nodelay { 
        // TCP_NODELAY: disable Nagle's algorithm for immediate sending
        r::set_tcp_nodelay(os, true)?; 
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = NetConfig::default();
        assert_eq!(config.tcp_nodelay, true);
        assert_eq!(config.recv_buf, Some(4 << 20));
        assert_eq!(config.send_buf, Some(4 << 20));
        assert_eq!(config.ipv6_only, Some(false));
    }
    
    #[test]
    fn test_low_latency_config() {
        let config = NetConfig::low_latency();
        assert_eq!(config.busy_poll, Some(50));
        assert_eq!(config.recv_buf, Some(256 * 1024));
        assert_eq!(config.poll_timeout_ms, Some(1));
    }
    
    #[test]
    fn test_high_throughput_config() {
        let config = NetConfig::high_throughput();
        assert_eq!(config.recv_buf, Some(16 << 20));
        assert_eq!(config.tcp_nodelay, false); // Nagle enabled for efficiency
        assert_eq!(config.tcp_backlog, Some(2048));
    }
    
    #[test]
    fn test_power_efficient_config() {
        let config = NetConfig::power_efficient();
        assert_eq!(config.busy_poll, None);
        assert_eq!(config.poll_timeout_ms, Some(100));
        assert_eq!(config.reuse_port, false);
    }
    
    #[test]
    fn test_config_clone() {
        let config1 = NetConfig::low_latency();
        let config2 = config1.clone();
        assert_eq!(config1, config2);
    }
}