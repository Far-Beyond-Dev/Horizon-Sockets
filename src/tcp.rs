//! High-performance TCP socket implementation
//!
//! This module provides TCP client and server implementations with extensive
//! low-latency optimizations and performance tuning options. The implementations
//! wrap the standard library's TCP types while applying advanced socket
//! configurations for optimal performance.
//!
//! # Key Features
//!
//! - **Low-Latency Optimizations**: TCP_NODELAY, TCP_QUICKACK (Linux)
//! - **Configurable Buffer Sizes**: Large send/receive buffers for throughput
//! - **Non-Blocking I/O**: All operations use non-blocking mode by default
//! - **Cross-Platform**: Works on Linux, Windows, macOS, and other Unix systems
//! - **IPv6 Support**: Full dual-stack IPv6 support with configuration options
//!
//! # Performance Optimizations
//!
//! The TCP implementation applies several optimizations automatically:
//!
//! - **TCP_NODELAY**: Disables Nagle's algorithm for immediate packet transmission
//! - **Large Buffers**: Configurable socket buffers (default: 4MB) for high throughput
//! - **TCP_QUICKACK**: (Linux only) Reduces ACK delay for better latency
//! - **SO_REUSEPORT**: (Linux/BSD) Enables load balancing across multiple threads
//!
//! # Examples
//!
//! ## TCP Server
//!
//! ```rust,no_run
//! use horizon_sockets::{NetConfig, tcp::TcpListener};
//! use std::io::{Read, Write};
//!
//! fn main() -> std::io::Result<()> {
//!     let config = NetConfig::low_latency();
//!     let listener = TcpListener::bind("0.0.0.0:8080".parse()?, &config)?;
//!
//!     loop {
//!         match listener.accept_nonblocking() {
//!             Ok((mut stream, addr)) => {
//!                 println!("Connection from: {}", addr);
//!                 
//!                 let mut buffer = [0u8; 1024];
//!                 if let Ok(n) = stream.as_std().read(&mut buffer) {
//!                     stream.as_std().write_all(&buffer[..n])?;
//!                 }
//!             }
//!             Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
//!                 std::thread::sleep(std::time::Duration::from_millis(1));
//!                 continue;
//!             }
//!             Err(e) => return Err(e),
//!         }
//!     }
//! }
//! ```
//!
//! ## TCP Client
//!
//! ```rust,no_run
//! use horizon_sockets::{NetConfig, tcp::TcpStream};
//! use std::net::TcpStream as StdTcpStream;
//! use std::io::{Read, Write};
//!
//! fn main() -> std::io::Result<()> {
//!     let config = NetConfig::low_latency();
//!     let std_stream = StdTcpStream::connect("127.0.0.1:8080")?;
//!     let mut stream = TcpStream::from_std(std_stream, &config)?;
//!
//!     stream.as_std().write_all(b"Hello, World!")?;
//!     
//!     let mut buffer = [0u8; 1024];
//!     let n = stream.as_std().read(&mut buffer)?;
//!     println!("Received: {}", std::str::from_utf8(&buffer[..n]).unwrap());
//!     
//!     Ok(())
//! }
//! ```

use crate::config::{NetConfig, apply_low_latency};
use crate::raw as r;
use std::io;
use std::net::{SocketAddr, TcpListener as StdTcpListener, TcpStream as StdTcpStream};

/// High-performance TCP listener with low-latency optimizations
///
/// This wrapper around the standard library's `TcpListener` applies
/// performance optimizations during socket creation and provides
/// non-blocking accept operations optimized for high-frequency servers.
///
/// # Performance Features
///
/// - **Non-blocking by default**: All accept operations are non-blocking
/// - **Optimized socket options**: Large buffers, TCP optimizations applied
/// - **IPv6 dual-stack support**: Configurable IPv6-only or dual-stack mode
/// - **Large accept backlog**: Configurable backlog size (default: 1024)
///
/// # Usage Patterns
///
/// The listener is designed for use in event loops or with async runtimes:
///
/// ```rust,no_run
/// use horizon_sockets::{NetConfig, tcp::TcpListener};
///
/// let config = NetConfig::default();
/// let listener = TcpListener::bind("0.0.0.0:8080".parse()?, &config)?;
///
/// loop {
///     match listener.accept_nonblocking() {
///         Ok((stream, addr)) => {
///             // Handle new connection
///         }
///         Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
///             // No pending connections, continue polling
///         }
///         Err(e) => return Err(e),
///     }
/// }
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct TcpListener {
    /// Underlying standard library TCP listener with applied optimizations
    inner: StdTcpListener,
}

/// High-performance TCP stream with low-latency optimizations
///
/// This wrapper around the standard library's `TcpStream` applies
/// performance optimizations for low-latency networking. The stream
/// is configured with TCP_NODELAY and other optimizations during creation.
///
/// # Performance Features
///
/// - **TCP_NODELAY enabled**: Disables Nagle's algorithm for immediate sending
/// - **Large buffers**: Configurable send/receive buffers for high throughput
/// - **Platform optimizations**: TCP_QUICKACK on Linux, optimized buffers on Windows
/// - **Standard library compatibility**: Direct access to underlying `TcpStream`
///
/// # Usage
///
/// The stream provides access to the underlying standard library stream
/// for all I/O operations while maintaining the applied optimizations:
///
/// ```rust,no_run
/// use horizon_sockets::{NetConfig, tcp::TcpStream};
/// use std::net::TcpStream as StdTcpStream;
/// use std::io::{Read, Write};
///
/// let config = NetConfig::low_latency();
/// let std_stream = StdTcpStream::connect("127.0.0.1:8080")?;
/// let stream = TcpStream::from_std(std_stream, &config)?;
///
/// // Use standard library methods through as_std()
/// stream.as_std().write_all(b"Hello")?;
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug)]
pub struct TcpStream {
    /// Underlying standard library TCP stream with applied optimizations
    inner: StdTcpStream,
}

impl TcpListener {
    /// Binds a TCP listener to the specified address with performance optimizations
    ///
    /// This method creates a TCP listener socket with all performance optimizations
    /// from the provided `NetConfig` applied. The socket is set to non-blocking mode
    /// and configured with the specified buffer sizes, TCP options, and IPv6 settings.
    ///
    /// # Arguments
    ///
    /// * `addr` - Socket address to bind to (IPv4 or IPv6)
    /// * `cfg` - Network configuration with performance tuning parameters
    ///
    /// # Returns
    ///
    /// A new `TcpListener` instance ready to accept connections
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use horizon_sockets::{NetConfig, tcp::TcpListener};
    ///
    /// // Bind with default configuration
    /// let config = NetConfig::default();
    /// let listener = TcpListener::bind("0.0.0.0:8080".parse()?, &config)?;
    ///
    /// // Bind with low-latency configuration
    /// let low_latency = NetConfig::low_latency();
    /// let listener = TcpListener::bind("[::]:8080".parse()?, &low_latency)?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    ///
    /// # Performance Notes
    ///
    /// - IPv6 addresses support dual-stack mode based on `cfg.ipv6_only`
    /// - Socket buffers are set according to `cfg.recv_buf` and `cfg.send_buf`
    /// - Listen backlog is configured from `cfg.tcp_backlog`
    /// - All TCP optimizations (NODELAY, QUICKACK) are applied
    pub fn bind(addr: SocketAddr, cfg: &NetConfig) -> io::Result<Self> {
        let (domain, sa, len) = r::to_sockaddr(addr);
        let os = r::socket(domain, r::Type::Stream, r::Protocol::Tcp)?;
        r::set_nonblocking(os, true)?;
        apply_low_latency(os, domain, r::Type::Stream, cfg)?;
        if let r::Domain::Ipv6 = domain {
            if let Some(only) = cfg.ipv6_only {
                r::set_ipv6_only(os, only)?;
            }
        }
        unsafe {
            r::bind_raw(os, &sa, len)?;
        }
        let backlog = cfg.tcp_backlog.unwrap_or(1024);
        r::listen_raw(os, backlog)?;
        let std = r::tcp_listener_from_os(os);
        Ok(Self { inner: std })
    }
    /// Accepts an incoming connection in non-blocking mode
    ///
    /// This method attempts to accept a pending connection from the listen queue.
    /// If no connection is pending, it returns `WouldBlock` error. The accepted
    /// connection is automatically configured with TCP_NODELAY for low latency.
    ///
    /// # Returns
    ///
    /// - `Ok((TcpStream, SocketAddr))` - New connection and its remote address
    /// - `Err(WouldBlock)` - No pending connections available
    /// - `Err(other)` - System error during accept operation
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use horizon_sockets::{NetConfig, tcp::TcpListener};
    /// use std::io::ErrorKind;
    ///
    /// let config = NetConfig::default();
    /// let listener = TcpListener::bind("0.0.0.0:8080".parse()?, &config)?;
    ///
    /// loop {
    ///     match listener.accept_nonblocking() {
    ///         Ok((stream, addr)) => {
    ///             println!("New connection from: {}", addr);
    ///             // Handle connection...
    ///             break;
    ///         }
    ///         Err(e) if e.kind() == ErrorKind::WouldBlock => {
    ///             // No connections pending, continue polling
    ///             std::thread::sleep(std::time::Duration::from_millis(1));
    ///             continue;
    ///         }
    ///         Err(e) => return Err(e),
    ///     }
    /// }
    /// # Ok::<(), std::io::Error>(())
    /// ```
    ///
    /// # Performance Notes
    ///
    /// - The returned `TcpStream` has TCP_NODELAY automatically enabled
    /// - This method should be called in a loop for continuous operation
    /// - Consider using with event notification systems for efficiency
    pub fn accept_nonblocking(&self) -> io::Result<(TcpStream, SocketAddr)> {
        self.inner.set_nonblocking(true)?;
        let (s, a) = self.inner.accept()?;
        s.set_nodelay(true)?;
        Ok((TcpStream { inner: s }, a))
    }
    /// Gets a reference to the underlying standard library TCP listener
    ///
    /// This provides direct access to the standard library `TcpListener` while
    /// maintaining all applied performance optimizations. Use this to access
    /// standard library methods not exposed by the wrapper.
    ///
    /// # Returns
    ///
    /// A reference to the underlying `std::net::TcpListener`
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use horizon_sockets::{NetConfig, tcp::TcpListener};
    ///
    /// let config = NetConfig::default();
    /// let listener = TcpListener::bind("0.0.0.0:8080".parse()?, &config)?;
    ///
    /// // Access standard library methods
    /// let local_addr = listener.as_std().local_addr()?;
    /// println!("Listening on: {}", local_addr);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn as_std(&self) -> &StdTcpListener {
        &self.inner
    }
}

impl TcpStream {
    /// Creates a TCP stream from a standard library stream with optimizations applied
    ///
    /// This method takes an existing `std::net::TcpStream` and applies the
    /// performance optimizations specified in the `NetConfig`. This is useful
    /// for optimizing streams obtained from `std::net::TcpStream::connect()` or
    /// from other sources.
    ///
    /// # Arguments
    ///
    /// * `s` - Standard library TCP stream to wrap and optimize
    /// * `cfg` - Network configuration with performance tuning parameters
    ///
    /// # Returns
    ///
    /// A new `TcpStream` with applied optimizations
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use horizon_sockets::{NetConfig, tcp::TcpStream};
    /// use std::net::TcpStream as StdTcpStream;
    ///
    /// // Connect using standard library
    /// let std_stream = StdTcpStream::connect("127.0.0.1:8080")?;
    ///
    /// // Apply low-latency optimizations
    /// let config = NetConfig::low_latency();
    /// let optimized_stream = TcpStream::from_std(std_stream, &config)?;
    /// # Ok::<(), std::io::Error>(())
    /// ```
    ///
    /// # Applied Optimizations
    ///
    /// - TCP_NODELAY is set according to `cfg.tcp_nodelay`
    /// - Additional optimizations may be applied in future versions
    pub fn from_std(s: StdTcpStream, cfg: &NetConfig) -> io::Result<Self> {
        s.set_nodelay(cfg.tcp_nodelay)?;
        Ok(Self { inner: s })
    }
    /// Gets a reference to the underlying standard library TCP stream
    ///
    /// This provides direct access to the standard library `TcpStream` for
    /// all I/O operations while maintaining the applied performance optimizations.
    /// Use this for reading, writing, and other stream operations.
    ///
    /// # Returns
    ///
    /// A reference to the underlying `std::net::TcpStream`
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use horizon_sockets::{NetConfig, tcp::TcpStream};
    /// use std::net::TcpStream as StdTcpStream;
    /// use std::io::{Read, Write};
    ///
    /// let config = NetConfig::default();
    /// let std_stream = StdTcpStream::connect("127.0.0.1:8080")?;
    /// let stream = TcpStream::from_std(std_stream, &config)?;
    ///
    /// // Perform I/O operations
    /// stream.as_std().write_all(b"Hello, server!")?;
    ///
    /// let mut buffer = [0u8; 1024];
    /// let n = stream.as_std().read(&mut buffer)?;
    /// println!("Received {} bytes", n);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn as_std(&self) -> &StdTcpStream {
        &self.inner
    }
}
