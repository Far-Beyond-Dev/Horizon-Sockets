//! Universal socket builder for creating both TCP and UDP sockets
//!
//! This module provides a unified builder interface similar to socket2 that can
//! create both TCP and UDP sockets with the same configuration methods. This
//! eliminates the need to learn separate builder APIs for different socket types.
//!
//! # Examples
//!
//! ```rust,no_run
//! use horizon_sockets::builder::SocketBuilder;
//! use std::net::TcpStream as StdTcpStream;
//!
//! // Create UDP socket
//! let udp_socket = SocketBuilder::new()
//!     .bind("0.0.0.0:8080")?
//!     .buffer_size(8 * 1024 * 1024)?
//!     .low_latency()?
//!     .udp()?;
//!
//! // Create TCP listener
//! let tcp_listener = SocketBuilder::new()
//!     .bind("0.0.0.0:8080")?
//!     .backlog(2048)?
//!     .nodelay(true)?
//!     .tcp_listener()?;
//!
//! // Create TCP stream from existing connection
//! let std_stream = StdTcpStream::connect("127.0.0.1:8080")?;
//! let tcp_stream = SocketBuilder::new()
//!     .from_std_tcp(std_stream)?
//!     .low_latency()?
//!     .tcp_stream()?;
//!
//! // Create dual-stack UDP socket
//! let dual_stack = SocketBuilder::new()
//!     .bind_dual_stack(8080)?
//!     .high_throughput()?
//!     .udp()?;
//! # Ok::<(), std::io::Error>(())
//! ```

use crate::config::NetConfig;
use crate::tcp::{TcpListener, TcpStream};
use crate::udp::Udp;
use std::io;
use std::net::{SocketAddr, TcpStream as StdTcpStream};

/// Universal socket builder for creating TCP and UDP sockets with method chaining
///
/// This builder provides a socket2-like interface that can create both TCP and UDP
/// sockets using the same configuration methods. It maintains all the high-performance
/// optimizations of Horizon Sockets while providing a unified, easy-to-use API.
///
/// # Design Philosophy
///
/// The builder uses the "consume and return" pattern where configuration methods
/// consume the builder and return a new configured instance. This allows for
/// method chaining while maintaining compile-time safety.
///
/// # Performance Features
///
/// - **Unified Configuration**: Same methods work for both TCP and UDP
/// - **Preset Configurations**: Built-in low-latency, high-throughput, and power-efficient presets
/// - **Platform Optimizations**: Automatically applies platform-specific optimizations
/// - **Type Safety**: Prevents invalid configurations at compile time
///
/// # Memory Management
///
/// The builder is lightweight and designed to be short-lived. It stores configuration
/// parameters and builds the final socket only when a terminal method is called.
#[derive(Debug, Clone)]
pub struct SocketBuilder {
    config: NetConfig,
    addr: Option<SocketAddr>,
    dual_stack_port: Option<u16>,
    std_tcp_stream: Option<StdTcpStream>,
}

impl SocketBuilder {
    /// Creates a new socket builder with default configuration
    ///
    /// The default configuration provides balanced performance suitable for
    /// most applications. Use preset methods like `low_latency()` or
    /// `high_throughput()` to optimize for specific use cases.
    pub fn new() -> Self {
        Self {
            config: NetConfig::default(),
            addr: None,
            dual_stack_port: None,
            std_tcp_stream: None,
        }
    }

    /// Binds the socket to a specific address
    ///
    /// This method accepts both IPv4 and IPv6 addresses in string format.
    /// The address will be parsed and validated during the bind operation.
    ///
    /// # Arguments
    /// * `addr` - Address to bind to (e.g., "127.0.0.1:8080", "[::1]:8080")
    ///
    /// # Examples
    /// ```rust,no_run
    /// use horizon_sockets::builder::SocketBuilder;
    ///
    /// let socket = SocketBuilder::new()
    ///     .bind("0.0.0.0:8080")?
    ///     .udp()?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn bind<A>(mut self, addr: A) -> io::Result<Self>
    where
        A: std::str::FromStr<Err = std::net::AddrParseError>,
    {
        self.addr = Some(addr.from_str().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid address: {}", e))
        })?);
        Ok(self)
    }

    /// Binds to a dual-stack IPv6 socket that accepts both IPv4 and IPv6 connections
    ///
    /// This is particularly useful on Windows where IPv6 sockets default to
    /// IPv6-only mode. This method explicitly enables dual-stack mode.
    ///
    /// # Arguments
    /// * `port` - Port number to bind to (0 for automatic assignment)
    pub fn bind_dual_stack(mut self, port: u16) -> io::Result<Self> {
        self.dual_stack_port = Some(port);
        self.config.ipv6_only = Some(false);
        Ok(self)
    }

    /// Configures the builder with an existing standard library TCP stream
    ///
    /// This is used when you have an existing TCP connection that you want to
    /// optimize with Horizon Sockets' performance features.
    ///
    /// # Arguments
    /// * `stream` - Existing standard library TCP stream
    pub fn from_std_tcp(mut self, stream: StdTcpStream) -> io::Result<Self> {
        self.std_tcp_stream = Some(stream);
        Ok(self)
    }

    /// Enables or disables TCP_NODELAY (Nagle's algorithm)
    ///
    /// When enabled (true), TCP packets are sent immediately rather than being
    /// buffered for efficiency. This reduces latency but may reduce throughput
    /// for small, frequent writes.
    ///
    /// **Default**: `true` (disabled Nagle's algorithm for low latency)
    pub fn nodelay(mut self, enable: bool) -> io::Result<Self> {
        self.config.tcp_nodelay = enable;
        Ok(self)
    }

    /// Enables or disables TCP_QUICKACK (Linux only)
    ///
    /// When enabled, reduces the delay before sending ACKs, which can improve
    /// request-response latency. This option is ignored on non-Linux platforms.
    ///
    /// **Default**: `true` on Linux
    pub fn quickack(mut self, enable: bool) -> io::Result<Self> {
        self.config.tcp_quickack = enable;
        Ok(self)
    }

    /// Enables SO_REUSEPORT for load balancing across multiple threads/processes
    ///
    /// This allows multiple sockets to bind to the same port for load distribution.
    /// Particularly useful for multi-threaded servers.
    ///
    /// **Platforms**: Linux, BSD (ignored on Windows)
    pub fn reuse_port(mut self, enable: bool) -> io::Result<Self> {
        self.config.reuse_port = enable;
        Ok(self)
    }

    /// Sets the TCP listen backlog size
    ///
    /// This controls the maximum number of pending connections in the accept queue.
    /// Larger values can handle connection bursts but use more memory.
    ///
    /// **Default**: 1024
    /// **Range**: Typically 1-65535 (OS dependent)
    pub fn backlog(mut self, backlog: i32) -> io::Result<Self> {
        self.config.tcp_backlog = Some(backlog);
        Ok(self)
    }

    /// Sets socket buffer sizes for both send and receive operations
    ///
    /// Larger buffers can improve throughput but may increase latency and
    /// memory usage. The kernel may adjust the actual size based on system limits.
    ///
    /// # Arguments
    /// * `size` - Buffer size in bytes
    ///
    /// # Recommendations
    /// - **Low latency**: 64KB - 512KB
    /// - **Balanced**: 1MB - 4MB
    /// - **High throughput**: 8MB - 64MB
    pub fn buffer_size(mut self, size: usize) -> io::Result<Self> {
        self.config.recv_buf = Some(size);
        self.config.send_buf = Some(size);
        Ok(self)
    }

    /// Sets the receive buffer size specifically
    pub fn recv_buffer_size(mut self, size: usize) -> io::Result<Self> {
        self.config.recv_buf = Some(size);
        Ok(self)
    }

    /// Sets the send buffer size specifically
    pub fn send_buffer_size(mut self, size: usize) -> io::Result<Self> {
        self.config.send_buf = Some(size);
        Ok(self)
    }

    /// Enables busy polling for the specified duration in microseconds (Linux only)
    ///
    /// Busy polling reduces latency by polling the network device for the specified
    /// time before falling back to interrupt-driven I/O. This trades CPU usage for
    /// reduced latency.
    ///
    /// # Arguments
    /// * `microseconds` - Polling duration (recommended range: 10-100μs)
    ///
    /// **Note**: Only effective on Linux with supported network drivers
    pub fn busy_poll(mut self, microseconds: u32) -> io::Result<Self> {
        self.config.busy_poll = Some(microseconds);
        Ok(self)
    }

    /// Sets Type of Service / DSCP marking for traffic prioritization
    ///
    /// This sets the TOS byte in IP headers for QoS and traffic classification.
    ///
    /// # Common Values
    /// - `0x10`: Low delay / minimize latency
    /// - `0x08`: High throughput / maximize throughput
    /// - `0x04`: High reliability
    /// - `0x02`: Minimize cost
    pub fn tos(mut self, tos: u32) -> io::Result<Self> {
        self.config.tos = Some(tos);
        Ok(self)
    }

    /// Configures IPv6-only mode or dual-stack mode
    ///
    /// # Arguments
    /// * `only` - `true` for IPv6-only, `false` for dual-stack (accepts IPv4 and IPv6)
    pub fn ipv6_only(mut self, only: bool) -> io::Result<Self> {
        self.config.ipv6_only = Some(only);
        Ok(self)
    }

    /// Sets IPv6 hop limit (equivalent to IPv4 TTL)
    ///
    /// Controls the maximum number of hops for IPv6 packets.
    ///
    /// **Default**: System default (typically 64)
    pub fn hop_limit(mut self, limit: i32) -> io::Result<Self> {
        self.config.hop_limit = Some(limit);
        Ok(self)
    }

    /// Sets the polling timeout for event operations
    ///
    /// This controls how long event loops wait for events before returning.
    ///
    /// # Arguments
    /// * `timeout_ms` - Timeout in milliseconds
    ///
    /// # Recommendations
    /// - **Low latency**: 1-10ms
    /// - **Balanced**: 10-50ms
    /// - **Power efficient**: 100ms+
    pub fn poll_timeout(mut self, timeout_ms: u64) -> io::Result<Self> {
        self.config.poll_timeout_ms = Some(timeout_ms);
        Ok(self)
    }

    /// Applies low-latency preset configuration
    ///
    /// Optimizes the socket for minimal latency at the cost of CPU usage:
    /// - Enables busy polling (50μs on Linux)
    /// - Uses smaller buffers (256KB) to minimize queuing delay
    /// - Enables all TCP latency optimizations (NODELAY, QUICKACK)
    /// - Sets low-delay DSCP marking
    /// - Uses aggressive polling timeout (1ms)
    pub fn low_latency(mut self) -> io::Result<Self> {
        let preset = NetConfig::low_latency();
        self.config.tcp_nodelay = preset.tcp_nodelay;
        self.config.tcp_quickack = preset.tcp_quickack;
        self.config.busy_poll = preset.busy_poll;
        self.config.recv_buf = preset.recv_buf;
        self.config.send_buf = preset.send_buf;
        self.config.tos = preset.tos;
        self.config.tcp_backlog = preset.tcp_backlog;
        self.config.poll_timeout_ms = preset.poll_timeout_ms;
        Ok(self)
    }

    /// Applies high-throughput preset configuration
    ///
    /// Optimizes the socket for maximum data transfer rates:
    /// - Uses large buffers (16MB) for high throughput
    /// - Disables busy polling to conserve CPU
    /// - Allows Nagle's algorithm for efficiency on bulk transfers
    /// - Sets high-throughput DSCP marking
    /// - Uses larger backlog (2048) for connection bursts
    pub fn high_throughput(mut self) -> io::Result<Self> {
        let preset = NetConfig::high_throughput();
        self.config.tcp_nodelay = preset.tcp_nodelay;
        self.config.tcp_quickack = preset.tcp_quickack;
        self.config.busy_poll = preset.busy_poll;
        self.config.recv_buf = preset.recv_buf;
        self.config.send_buf = preset.send_buf;
        self.config.tos = preset.tos;
        self.config.tcp_backlog = preset.tcp_backlog;
        self.config.poll_timeout_ms = preset.poll_timeout_ms;
        Ok(self)
    }

    /// Applies power-efficient preset configuration
    ///
    /// Optimizes the socket for minimal CPU and power usage:
    /// - Uses moderate buffers (512KB)
    /// - Disables busy polling and other CPU-intensive optimizations
    /// - Uses longer polling timeouts to reduce wakeups
    /// - Simplifies socket management to reduce overhead
    pub fn power_efficient(mut self) -> io::Result<Self> {
        let preset = NetConfig::power_efficient();
        self.config.tcp_nodelay = preset.tcp_nodelay;
        self.config.tcp_quickack = preset.tcp_quickack;
        self.config.busy_poll = preset.busy_poll;
        self.config.recv_buf = preset.recv_buf;
        self.config.send_buf = preset.send_buf;
        self.config.reuse_port = preset.reuse_port;
        self.config.tcp_backlog = preset.tcp_backlog;
        self.config.poll_timeout_ms = preset.poll_timeout_ms;
        Ok(self)
    }

    /// Builds a UDP socket with the configured settings
    ///
    /// # Returns
    /// A configured `Udp` socket ready for datagram operations
    ///
    /// # Errors
    /// - No address specified with `bind()` or `bind_dual_stack()`
    /// - Address is invalid or unavailable
    /// - Socket creation fails
    pub fn udp(self) -> io::Result<Udp> {
        if let Some(port) = self.dual_stack_port {
            Udp::bind_dual_stack(port, &self.config)
        } else if let Some(addr) = self.addr {
            Udp::bind(addr, &self.config)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Must specify address with bind() or bind_dual_stack()",
            ))
        }
    }

    /// Builds a TCP listener with the configured settings
    ///
    /// # Returns
    /// A configured `TcpListener` ready to accept connections
    ///
    /// # Errors
    /// - No address specified with `bind()`
    /// - Address is invalid or unavailable
    /// - Listener creation fails
    pub fn tcp_listener(self) -> io::Result<TcpListener> {
        if let Some(addr) = self.addr {
            TcpListener::bind(addr, &self.config)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Must specify address with bind() for TCP listener",
            ))
        }
    }

    /// Builds a TCP stream with the configured settings
    ///
    /// This requires a standard library TCP stream provided via `from_std_tcp()`.
    ///
    /// # Returns
    /// A configured `TcpStream` ready for I/O operations
    ///
    /// # Errors
    /// - No standard stream provided with `from_std_tcp()`
    /// - Stream configuration fails
    pub fn tcp_stream(self) -> io::Result<TcpStream> {
        if let Some(std_stream) = self.std_tcp_stream {
            TcpStream::from_std(std_stream, &self.config)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Must provide standard stream with from_std_tcp() for TCP stream",
            ))
        }
    }
}

impl Default for SocketBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let builder = SocketBuilder::new();
        assert!(builder.addr.is_none());
        assert!(builder.dual_stack_port.is_none());
    }

    #[test]
    fn test_bind_method() {
        let builder = SocketBuilder::new()
            .bind("127.0.0.1:8080")
            .unwrap();
        assert!(builder.addr.is_some());
        assert_eq!(builder.addr.unwrap().port(), 8080);
    }

    #[test]
    fn test_dual_stack_bind() {
        let builder = SocketBuilder::new()
            .bind_dual_stack(8080)
            .unwrap();
        assert_eq!(builder.dual_stack_port, Some(8080));
        assert_eq!(builder.config.ipv6_only, Some(false));
    }

    #[test]
    fn test_configuration_chaining() {
        let builder = SocketBuilder::new()
            .nodelay(false).unwrap()
            .buffer_size(1024 * 1024).unwrap()
            .backlog(2048).unwrap();
        
        assert_eq!(builder.config.tcp_nodelay, false);
        assert_eq!(builder.config.recv_buf, Some(1024 * 1024));
        assert_eq!(builder.config.tcp_backlog, Some(2048));
    }

    #[test]
    fn test_preset_configurations() {
        let low_lat = SocketBuilder::new()
            .low_latency()
            .unwrap();
        assert!(low_lat.config.busy_poll.is_some());
        assert_eq!(low_lat.config.tcp_nodelay, true);

        let high_tp = SocketBuilder::new()
            .high_throughput()
            .unwrap();
        assert_eq!(high_tp.config.tcp_nodelay, false); // Nagle enabled for efficiency

        let power = SocketBuilder::new()
            .power_efficient()
            .unwrap();
        assert_eq!(power.config.busy_poll, None);
    }

    #[test]
    fn test_udp_build_requires_address() {
        let result = SocketBuilder::new().udp();
        assert!(result.is_err());
        
        let result = SocketBuilder::new()
            .bind("127.0.0.1:0")
            .unwrap()
            .udp();
        assert!(result.is_ok());
    }

    #[test]
    fn test_tcp_listener_build_requires_address() {
        let result = SocketBuilder::new().tcp_listener();
        assert!(result.is_err());
        
        let result = SocketBuilder::new()
            .bind("127.0.0.1:0")
            .unwrap()
            .tcp_listener();
        assert!(result.is_ok());
    }
}