use std::io;
use std::net::{SocketAddr, UdpSocket as StdUdpSocket};
use crate::config::{NetConfig, apply_low_latency};
use crate::raw as r;

pub struct Udp { inner: StdUdpSocket }

impl Udp {
    pub fn bind(addr: SocketAddr, cfg: &NetConfig) -> io::Result<Self> {
        let (domain, sa, len) = r::to_sockaddr(addr);
        let os = r::socket(domain, r::Type::Dgram, r::Protocol::Udp)?;
        r::set_nonblocking(os, true)?;
        apply_low_latency(os, domain, r::Type::Dgram, cfg)?;
        // Default to dual-stack on IPv6 unless explicitly set
        if let r::Domain::Ipv6 = domain { r::set_ipv6_only(os, cfg.ipv6_only.unwrap_or(false))?; }
        unsafe { r::bind_raw(os, &sa, len)?; }
        let std = unsafe { r::udp_from_os(os) };
        Ok(Self { inner: std })
    }

    /// Bind dual-stack on IPv6 any with optional v6only=false (Windows often defaults to true)
    pub fn bind_dual_stack(port: u16, cfg: &NetConfig) -> io::Result<Self> {
        let any6: SocketAddr = "[::]:0".parse().unwrap();
        let (domain, mut sa, len) = r::to_sockaddr(any6);
        if let r::SockAddr::V6(ref mut s6) = sa { s6.sin6_port = (port as u16).to_be(); }
        let os = r::socket(r::Domain::Ipv6, r::Type::Dgram, r::Protocol::Udp)?;
        r::set_nonblocking(os, true)?;
        apply_low_latency(os, r::Domain::Ipv6, r::Type::Dgram, cfg)?;
        r::set_ipv6_only(os, cfg.ipv6_only.unwrap_or(false))?;
        unsafe { r::bind_raw(os, &sa, len)?; }
        let std = unsafe { r::udp_from_os(os) };
        Ok(Self { inner: std })
    }

    pub fn socket(&self) -> &StdUdpSocket { &self.inner }

    /// Batch receive: on Linux use recvmmsg if available; otherwise recv_from loop.
    pub fn recv_batch(&self, bufs: &mut [Vec<u8>], addrs: &mut [SocketAddr]) -> io::Result<usize> {
        cfg_if::cfg_if! {
            if #[cfg(any(target_os = "linux", target_os = "android"))] {
                unsafe { recv_batch_linux(self, bufs, addrs) }
            } else {
                let mut n = 0; 
                for i in 0..bufs.len() { 
                    match self.inner.recv_from(&mut bufs[i]) { 
                        Ok((len, addr)) => { addrs[i] = addr; bufs[i].truncate(len); n += 1; },
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(e) => return Err(e),
                    }
                }
                Ok(n)
            }
        }
    }

    pub fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> { self.inner.send_to(buf, addr) }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::os::unix::io::AsRawFd;
#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn recv_batch_linux(sock: &Udp, bufs: &mut [Vec<u8>], addrs: &mut [SocketAddr]) -> io::Result<usize> {
    use libc::*;
    let fd = sock.inner.as_raw_fd();
    let max = bufs.len().min(addrs.len());

    let mut hdrs: Vec<mmsghdr> = Vec::with_capacity(max);
    let mut iovecs: Vec<iovec> = Vec::with_capacity(max);
    let mut addrs_raw: Vec<sockaddr_storage> = Vec::with_capacity(max);

    unsafe {
        hdrs.set_len(max);
        iovecs.set_len(max);
        addrs_raw.set_len(max);
    }

    for i in 0..max {
        let buf = &mut bufs[i];
        if buf.capacity() == 0 { buf.reserve_exact(2048); buf.resize(2048, 0); }
        let iov = iovec { iov_base: buf.as_mut_ptr() as _, iov_len: buf.len() };
        iovecs[i] = iov;
        hdrs[i].msg_hdr = msghdr {
            msg_name: &mut addrs_raw[i] as *mut _ as *mut _,
            msg_namelen: std::mem::size_of::<sockaddr_storage>() as _,
            msg_iov: &mut iovecs[i] as *mut _ ,
            msg_iovlen: 1,
            msg_control: std::ptr::null_mut(),
            msg_controllen: 0,
            msg_flags: 0,
        };
        hdrs[i].msg_len = 0;
    }

    let rc = unsafe { recvmmsg(fd, hdrs.as_mut_ptr(), max as u32, MSG_DONTWAIT, std::ptr::null_mut()) };
    if rc < 0 { return Err(std::io::Error::last_os_error()); }
    let n = rc as usize;

    for i in 0..n {
        let len = hdrs[i].msg_len as usize;
        bufs[i].truncate(len);
        // Convert sockaddr_storage -> SocketAddr
        let ss = &addrs_raw[i];
        let sa = unsafe { &*(ss as *const _ as *const sockaddr) };
        let addr = if sa.sa_family as i32 == AF_INET { 
            let sin = unsafe { &*(ss as *const _ as *const sockaddr_in) };
            let ip = std::net::Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr));
            let port = u16::from_be(sin.sin_port);
            SocketAddr::new(ip.into(), port)
        } else {
            let sin6 = unsafe { &*(ss as *const _ as *const sockaddr_in6) };
            let ip = std::net::Ipv6Addr::from(sin6.sin6_addr.s6_addr);
            let port = u16::from_be(sin6.sin6_port);
            SocketAddr::new(ip.into(), port)
        };
        addrs[i] = addr;
    }
    Ok(n)
}
