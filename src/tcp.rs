use std::io;
use std::net::{SocketAddr, TcpListener as StdTcpListener, TcpStream as StdTcpStream};
use crate::config::{NetConfig, apply_low_latency};
use crate::raw as r;

pub struct TcpListener { inner: StdTcpListener }
pub struct TcpStream { inner: StdTcpStream }

impl TcpListener {
    pub fn bind(addr: SocketAddr, cfg: &NetConfig) -> io::Result<Self> {
        let (domain, sa, len) = r::to_sockaddr(addr);
        let os = r::socket(domain, r::Type::Stream, r::Protocol::Tcp)?;
        r::set_nonblocking(os, true)?;
        apply_low_latency(os, domain, r::Type::Stream, cfg)?;
        if let r::Domain::Ipv6 = domain { if let Some(only) = cfg.ipv6_only { r::set_ipv6_only(os, only)?; } }
        unsafe { r::bind_raw(os, &sa, len)?; }
        let backlog = cfg.tcp_backlog.unwrap_or(1024);
        r::listen_raw(os, backlog)?;
        let std = unsafe { r::tcp_listener_from_os(os) };
        Ok(Self { inner: std })
    }
    pub fn accept_nonblocking(&self) -> io::Result<(TcpStream, SocketAddr)> {
        self.inner.set_nonblocking(true)?;
        let (s, a) = self.inner.accept()?; s.set_nodelay(true)?; Ok((TcpStream{ inner: s }, a))
    }
    pub fn as_std(&self) -> &StdTcpListener { &self.inner }
}

impl TcpStream {
    pub fn from_std(s: StdTcpStream, cfg: &NetConfig) -> io::Result<Self> { s.set_nodelay(cfg.tcp_nodelay)?; Ok(Self{ inner: s }) }
    pub fn as_std(&self) -> &StdTcpStream { &self.inner }
}