use crate::config::{NetConfig, apply_low_latency};
use crate::raw as r;
use std::io;
use std::net::{SocketAddr, TcpListener as StdTcpListener, TcpStream as StdTcpStream};

/// High-performance TCP listener with low-latency optimizations
#[derive(Debug)]
pub struct TcpListener {
    inner: StdTcpListener,
}

/// High-performance TCP stream with low-latency optimizations
#[derive(Debug)]
pub struct TcpStream {
    inner: StdTcpStream,
}

impl TcpListener {
    /// Bind TCP listener to address with low-latency configuration
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
    /// Accept incoming connection in non-blocking mode
    pub fn accept_nonblocking(&self) -> io::Result<(TcpStream, SocketAddr)> {
        self.inner.set_nonblocking(true)?;
        let (s, a) = self.inner.accept()?;
        s.set_nodelay(true)?;
        Ok((TcpStream { inner: s }, a))
    }
    /// Get reference to underlying standard library TCP listener
    pub fn as_std(&self) -> &StdTcpListener {
        &self.inner
    }
}

impl TcpStream {
    /// Create from standard library TCP stream with low-latency configuration
    pub fn from_std(s: StdTcpStream, cfg: &NetConfig) -> io::Result<Self> {
        s.set_nodelay(cfg.tcp_nodelay)?;
        Ok(Self { inner: s })
    }
    /// Get reference to underlying standard library TCP stream
    pub fn as_std(&self) -> &StdTcpStream {
        &self.inner
    }
}
