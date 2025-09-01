//! Low-level socket operations and platform abstractions
//!
//! This module provides platform-specific socket operations and abstractions that
//! form the foundation for high-performance networking. It handles the differences
//! between Unix-like systems (Linux, macOS, BSD) and Windows, providing a unified
//! interface for socket creation, configuration, and optimization.
//!
//! # Platform Support
//!
//! ## Unix Systems (Linux, macOS, BSD, Android)
//! - Uses POSIX socket APIs (`socket`, `bind`, `listen`, etc.)
//! - File descriptor-based socket handles
//! - Support for advanced options like SO_BUSY_POLL (Linux)
//! - Native IPv6 dual-stack support
//!
//! ## Windows
//! - Uses WinSock2 APIs with automatic WSA initialization
//! - SOCKET handle-based operations
//! - Enhanced IOCP preparation for async operations
//! - Comprehensive socket option support
//!
//! # Key Abstractions
//!
//! - **Domain**: IP protocol family (IPv4 vs IPv6)
//! - **Type**: Socket type (Stream for TCP, Dgram for UDP)
//! - **Protocol**: Transport protocol (TCP, UDP)
//! - **SockAddr**: Platform-specific socket address storage
//!
//! # Safety
//!
//! This module contains `unsafe` code for:
//! - Raw socket system calls
//! - Memory management of socket addresses
//! - Platform-specific socket option manipulation
//!
//! All `unsafe` operations are carefully encapsulated within safe interfaces.

use std::io;
use std::net::SocketAddr;

/// IP protocol domain for sockets
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Domain {
    /// IPv4 protocol
    Ipv4,
    /// IPv6 protocol
    Ipv6,
}

/// Socket type for protocol communication
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Type {
    /// TCP stream socket
    Stream,
    /// UDP datagram socket
    Dgram,
}

/// Transport protocol for sockets
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Protocol {
    /// TCP protocol
    Tcp,
    /// UDP protocol
    Udp,
}

cfg_if::cfg_if! {
    if #[cfg(unix)] {
        use std::os::unix::io::{RawFd, FromRawFd, AsRawFd};
        pub type OsSocket = RawFd;

        /// Platform-specific socket address storage
        #[allow(non_camel_case_types)]
        #[derive(Debug)]
        pub enum SockAddr {
            /// IPv4 socket address
            V4(libc::sockaddr_in),
            /// IPv6 socket address
            V6(libc::sockaddr_in6),
        }

        /// Convert SocketAddr to platform-specific socket address
        pub fn to_sockaddr(addr: SocketAddr) -> (Domain, SockAddr, libc::socklen_t) {
            match addr {
                SocketAddr::V4(a) => {
                    let mut s: libc::sockaddr_in = unsafe { std::mem::zeroed() };
                    s.sin_family = libc::AF_INET as _;
                    s.sin_port = a.port().to_be();
                    s.sin_addr = libc::in_addr { s_addr: u32::from_ne_bytes(a.ip().octets()).to_be() };
                    (Domain::Ipv4, SockAddr::V4(s), std::mem::size_of::<libc::sockaddr_in>() as _)
                }
                SocketAddr::V6(a) => {
                    let mut s: libc::sockaddr_in6 = unsafe { std::mem::zeroed() };
                    s.sin6_family = libc::AF_INET6 as _;
                    s.sin6_port = a.port().to_be();
                    s.sin6_flowinfo = a.flowinfo();
                    s.Anonymous.sin6_scope_id = a.scope_id();
                    s.sin6_addr = libc::in6_addr { s6_addr: a.ip().octets() };
                    (Domain::Ipv6, SockAddr::V6(s), std::mem::size_of::<libc::sockaddr_in6>() as _)
                }
            }
        }

        /// Raw bind operation for socket to address
        pub unsafe fn bind_raw(os: OsSocket, sa: &SockAddr, len: libc::socklen_t) -> io::Result<()> {
            let (ptr, l) = match sa {
                SockAddr::V4(s) => (s as *const _ as *const libc::sockaddr, len),
                SockAddr::V6(s) => (s as *const _ as *const libc::sockaddr, len),
            };
            if libc::bind(os, ptr, l) != 0 { return Err(io::Error::last_os_error()); }
            Ok(())
        }

        /// Create a new socket with specified domain and type
        pub fn socket(domain: Domain, ty: Type, proto: Protocol) -> io::Result<OsSocket> {
            let d = match domain { Domain::Ipv4 => libc::AF_INET, Domain::Ipv6 => libc::AF_INET6 };
            let t = match ty { Type::Stream => libc::SOCK_STREAM, Type::Dgram => libc::SOCK_DGRAM };
            let p = match proto { Protocol::Tcp => libc::IPPROTO_TCP, Protocol::Udp => libc::IPPROTO_UDP };
            let fd = unsafe { libc::socket(d, t | libc::SOCK_CLOEXEC, p) };
            if fd < 0 { return Err(io::Error::last_os_error()); }
            Ok(fd)
        }

        /// Set socket non-blocking mode
        pub fn set_nonblocking(os: OsSocket, on: bool) -> io::Result<()> {
            unsafe {
                let flags = libc::fcntl(os, libc::F_GETFL);
                if flags < 0 { return Err(io::Error::last_os_error()); }
                let nb = if on { flags | libc::O_NONBLOCK } else { flags & !libc::O_NONBLOCK };
                if libc::fcntl(os, libc::F_SETFL, nb) != 0 { return Err(io::Error::last_os_error()); }
                Ok(())
            }
        }

        /// Start listening on socket with specified backlog
        pub fn listen_raw(os: OsSocket, backlog: i32) -> io::Result<()> { if unsafe { libc::listen(os, backlog) } != 0 { Err(io::Error::last_os_error()) } else { Ok(()) } }

        /// Set socket receive buffer size
        pub fn set_recv_buffer(os: OsSocket, sz: i32) -> io::Result<()> { setsockopt_int(os, libc::SOL_SOCKET, libc::SO_RCVBUF, sz) }
        /// Set socket send buffer size
        pub fn set_send_buffer(os: OsSocket, sz: i32) -> io::Result<()> { setsockopt_int(os, libc::SOL_SOCKET, libc::SO_SNDBUF, sz) }
        /// Enable port reuse for multiple binds
        pub fn set_reuse_port(os: OsSocket, on: bool) -> io::Result<()> { setsockopt_int(os, libc::SOL_SOCKET, libc::SO_REUSEPORT, on as i32) }
        /// Set IPv4 Type of Service for low-latency routing
        pub fn set_tos_v4(os: OsSocket, tos: i32) -> io::Result<()> { setsockopt_int(os, libc::IPPROTO_IP, libc::IP_TOS, tos) }
        /// Set IPv6 Traffic Class for low-latency routing
        pub fn set_tos_v6(os: OsSocket, tc: i32) -> io::Result<()> { setsockopt_int(os, libc::IPPROTO_IPV6, libc::IPV6_TCLASS, tc) }
        /// Configure IPv6-only mode (disable dual-stack)
        pub fn set_ipv6_only(os: OsSocket, only: bool) -> io::Result<()> { setsockopt_int(os, libc::IPPROTO_IPV6, libc::IPV6_V6ONLY, only as i32) }
        /// Set IPv6 hop limit for packet routing
        pub fn set_ipv6_hop_limit(os: OsSocket, hops: i32) -> io::Result<()> { setsockopt_int(os, libc::IPPROTO_IPV6, libc::IPV6_UNICAST_HOPS, hops) }
        /// Disable TCP Nagle algorithm for low latency
        pub fn set_tcp_nodelay(os: OsSocket, on: bool) -> io::Result<()> { setsockopt_int(os, libc::IPPROTO_TCP, libc::TCP_NODELAY, on as i32) }
        /// Enable TCP quick ACK for low latency
        pub fn set_tcp_quickack(os: OsSocket, on: bool) -> io::Result<()> { setsockopt_int(os, libc::IPPROTO_TCP, 12, on as i32) }
        /// Enable busy polling for minimal latency
        pub fn set_busy_poll(os: OsSocket, usec: u32) -> io::Result<()> { setsockopt_int(os, libc::SOL_SOCKET, 46, usec as i32) }

        fn setsockopt_int(fd: RawFd, level: i32, opt: i32, val: i32) -> io::Result<()> {
            let v = val as libc::c_int;
            let rc = unsafe { libc::setsockopt(fd, level, opt, &v as *const _ as _, std::mem::size_of::<libc::c_int>() as _) };
            if rc != 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
        }

        /// Convert OS socket to std UDP socket
        pub unsafe fn udp_from_os(fd: RawFd) -> std::net::UdpSocket { std::net::UdpSocket::from_raw_fd(fd) }
        /// Convert OS socket to std TCP listener
        pub unsafe fn tcp_listener_from_os(fd: RawFd) -> std::net::TcpListener { std::net::TcpListener::from_raw_fd(fd) }
        /// Convert OS socket to std TCP stream
        pub unsafe fn tcp_stream_from_os(fd: RawFd) -> std::net::TcpStream { std::net::TcpStream::from_raw_fd(fd) }

    } else {
        // Windows
        use std::sync::Once;
        use windows_sys::Win32::Networking::WinSock::*;
        use std::os::windows::io::{RawSocket, FromRawSocket};
        /// Windows socket handle type
        pub type OsSocket = RawSocket; // SOCKET

        static START: Once = Once::new();
        fn ensure_wsa() {
            START.call_once(|| unsafe {
                let mut data: WSADATA = std::mem::zeroed();
                let rc = WSAStartup(0x202, &mut data); // MAKEWORD(2,2)
                if rc != 0 { panic!("WSAStartup failed: {}", rc); }
            });
        }

        /// Platform-specific socket address storage
        #[allow(non_camel_case_types, missing_debug_implementations)]
        pub enum SockAddr {
            /// IPv4 socket address
            V4(SOCKADDR_IN),
            /// IPv6 socket address
            V6(SOCKADDR_IN6),
        }

        /// Convert SocketAddr to platform-specific socket address
        pub fn to_sockaddr(addr: SocketAddr) -> (Domain, SockAddr, i32) {
            match addr {
                SocketAddr::V4(a) => {
                    let mut s: SOCKADDR_IN = unsafe { std::mem::zeroed() };
                    s.sin_family = AF_INET as _;
                    s.sin_port = a.port().to_be();
                    s.sin_addr = IN_ADDR { S_un: IN_ADDR_0 { S_addr: u32::from_be_bytes(a.ip().octets()) } };
                    (Domain::Ipv4, SockAddr::V4(s), std::mem::size_of::<SOCKADDR_IN>() as _)
                }
                SocketAddr::V6(a) => {
                    let mut s: SOCKADDR_IN6 = unsafe { std::mem::zeroed() };
                    s.sin6_family = AF_INET6 as _;
                    s.sin6_port = a.port().to_be();
                    s.sin6_flowinfo = a.flowinfo();
                    s.Anonymous.sin6_scope_id = a.scope_id();
                    s.sin6_addr = IN6_ADDR { u: IN6_ADDR_0 { Byte: a.ip().octets() } };
                    (Domain::Ipv6, SockAddr::V6(s), std::mem::size_of::<SOCKADDR_IN6>() as _)
                }
            }
        }

        /// Raw bind operation for socket to address
        pub unsafe fn bind_raw(os: OsSocket, sa: &SockAddr, len: i32) -> io::Result<()> {
            ensure_wsa();
            let (ptr, l) = match sa {
                SockAddr::V4(s) => (s as *const _ as *const SOCKADDR, len),
                SockAddr::V6(s) => (s as *const _ as *const SOCKADDR, len),
            };
            let rc = unsafe { bind(os as usize, ptr, l) };
            if rc != 0 { return Err(io::Error::from_raw_os_error(unsafe { WSAGetLastError() })); }
            Ok(())
        }

        /// Create a new socket with specified domain and type
        pub fn socket(domain: Domain, ty: Type, _proto: Protocol) -> io::Result<OsSocket> {
            ensure_wsa();
            let d = match domain { Domain::Ipv4 => AF_INET, Domain::Ipv6 => AF_INET6 } as i32;
            let t = match ty { Type::Stream => SOCK_STREAM, Type::Dgram => SOCK_DGRAM } as i32;
            let s = unsafe { WSASocketW(d, t, 0, std::ptr::null_mut(), 0, WSA_FLAG_OVERLAPPED) };
            if s == INVALID_SOCKET { return Err(io::Error::from_raw_os_error(unsafe { WSAGetLastError() })); }
            Ok(s as _)
        }

        /// Set socket non-blocking mode
        pub fn set_nonblocking(os: OsSocket, on: bool) -> io::Result<()> {
            ensure_wsa();

            let mut nb: u32 = if on {1} else {0};
            if unsafe { ioctlsocket(os as usize, FIONBIO, &mut nb) } != 0 { return Err(io::Error::from_raw_os_error(unsafe { WSAGetLastError() })); }
            Ok(())
        }

        /// Start listening on socket with specified backlog
        pub fn listen_raw(os: OsSocket, backlog: i32) -> io::Result<()> { if unsafe { listen(os as usize, backlog) } != 0 { Err(io::Error::from_raw_os_error(unsafe { WSAGetLastError() })) } else { Ok(()) } }

        fn setsockopt_int(socket: OsSocket, level: i32, opt: i32, val: i32) -> io::Result<()> {
            unsafe {
                let rc = setsockopt(socket as usize, level, opt, &val as *const _ as _, std::mem::size_of::<i32>() as _);
                if rc != 0 { Err(io::Error::from_raw_os_error(WSAGetLastError())) } else { Ok(()) }
            }
        }
        /// Set socket receive buffer size
        pub fn set_recv_buffer(os: OsSocket, sz: i32) -> io::Result<()> { setsockopt_int(os, SOL_SOCKET as _, SO_RCVBUF as _, sz) }
        /// Set socket send buffer size
        pub fn set_send_buffer(os: OsSocket, sz: i32) -> io::Result<()> { setsockopt_int(os, SOL_SOCKET as _, SO_SNDBUF as _, sz) }
        /// Set IPv4 Type of Service for low-latency routing
        pub fn set_tos_v4(os: OsSocket, tos: i32) -> io::Result<()> { setsockopt_int(os, IPPROTO_IP as _, IP_TOS as _, tos) }
        /// Set IPv6 Traffic Class for low-latency routing
        pub fn set_tos_v6(os: OsSocket, tc: i32) -> io::Result<()> { setsockopt_int(os, IPPROTO_IPV6 as _, IPV6_TCLASS as _, tc) }
        /// Configure IPv6-only mode (disable dual-stack)
        pub fn set_ipv6_only(os: OsSocket, only: bool) -> io::Result<()> { setsockopt_int(os, IPPROTO_IPV6 as _, IPV6_V6ONLY as _, if only {1} else {0}) }
        /// Set IPv6 hop limit for packet routing
        pub fn set_ipv6_hop_limit(os: OsSocket, hops: i32) -> io::Result<()> { setsockopt_int(os, IPPROTO_IPV6 as _, IPV6_UNICAST_HOPS as _, hops) }
        /// Disable TCP Nagle algorithm for low latency
        pub fn set_tcp_nodelay(os: OsSocket, on: bool) -> io::Result<()> { setsockopt_int(os, IPPROTO_TCP as _, TCP_NODELAY as _, if on {1} else {0}) }
        /// Enable TCP quick ACK (no-op on Windows)
        pub fn set_tcp_quickack(_os: OsSocket, _on: bool) -> io::Result<()> { Ok(()) /* not available on Windows */ }
        /// Enable port reuse (no-op on Windows)
        pub fn set_reuse_port(_os: OsSocket, _on: bool) -> io::Result<()> { Ok(()) /* not applicable */ }
        /// Enable busy polling for minimal latency (no-op on Windows)
        pub fn set_busy_poll(_os: OsSocket, _usec: u32) -> io::Result<()> { Ok(()) /* not applicable */ }

        /// Convert OS socket to std UDP socket
        pub fn udp_from_os(s: OsSocket) -> std::net::UdpSocket { unsafe { std::net::UdpSocket::from_raw_socket(s) } }
        /// Convert OS socket to std TCP listener
        pub fn tcp_listener_from_os(s: OsSocket) -> std::net::TcpListener { unsafe { std::net::TcpListener::from_raw_socket(s) } }
        /// Convert OS socket to std TCP stream
        pub fn tcp_stream_from_os(s: OsSocket) -> std::net::TcpStream { unsafe { std::net::TcpStream::from_raw_socket(s) } }
    }
}
