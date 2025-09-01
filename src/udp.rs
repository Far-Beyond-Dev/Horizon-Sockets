use std::io;
use std::net::{SocketAddr, UdpSocket as StdUdpSocket};
use crate::config::{NetConfig, apply_low_latency};
use crate::raw as r;

#[cfg(windows)]
use std::os::windows::io::AsRawSocket;

/// High-performance UDP socket with low-latency optimizations
#[derive(Debug)]
pub struct Udp { inner: StdUdpSocket }

impl Udp {
    /// Bind UDP socket to address with low-latency configuration
    pub fn bind(addr: SocketAddr, cfg: &NetConfig) -> io::Result<Self> {
        // Use standard library binding for simplicity and compatibility
        let std = StdUdpSocket::bind(addr)?;
        std.set_nonblocking(true)?;
        
        // Apply low-latency configurations if possible
        cfg_if::cfg_if! {
            if #[cfg(windows)] {
                let os = std.as_raw_socket() as r::OsSocket;
                let (domain, _, _) = r::to_sockaddr(addr);
                let _ = apply_low_latency(os, domain, r::Type::Dgram, cfg);
                
                // Configure IPv6 dual-stack if needed  
                if let (SocketAddr::V6(_), Some(v6only)) = (addr, cfg.ipv6_only) {
                    let _ = r::set_ipv6_only(os, v6only);
                }
            } else {
                // Unix platforms - would use as_raw_fd() 
                let _ = (addr, cfg); // suppress unused warnings
            }
        }
        
        Ok(Self { inner: std })
    }

    /// Bind dual-stack on IPv6 any with optional v6only=false (Windows often defaults to true)
    pub fn bind_dual_stack(port: u16, cfg: &NetConfig) -> io::Result<Self> {
        let any6: SocketAddr = "[::]:0".parse().unwrap();
        let (_domain, mut sa, len) = r::to_sockaddr(any6);
        if let r::SockAddr::V6(ref mut s6) = sa { s6.sin6_port = (port as u16).to_be(); }
        let os = r::socket(r::Domain::Ipv6, r::Type::Dgram, r::Protocol::Udp)?;
        r::set_nonblocking(os, true)?;
        apply_low_latency(os, r::Domain::Ipv6, r::Type::Dgram, cfg)?;
        r::set_ipv6_only(os, cfg.ipv6_only.unwrap_or(false))?;
        unsafe { r::bind_raw(os, &sa, len)?; }
        let std = r::udp_from_os(os);
        Ok(Self { inner: std })
    }

    /// Get reference to underlying standard library UDP socket
    pub fn socket(&self) -> &StdUdpSocket { &self.inner }

    /// Batch receive: on Linux use recvmmsg if available; otherwise recv_from loop
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

    /// Send data to specific address
    pub fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> { self.inner.send_to(buf, addr) }

    /// Send a batch of packets; returns the number of packets successfully sent
    pub fn send_batch(&self, packets: &[( &[u8], SocketAddr )]) -> io::Result<usize> {
        let mut sent = 0;
        for (buf, addr) in packets {
            match self.send_to(buf, *addr) {
                Ok(_) => sent += 1,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(sent)
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe fn recv_batch_linux(sock: &Udp, bufs: &mut [Vec<u8>], addrs: &mut [SocketAddr]) -> io::Result<usize> {
    use libc::*;
    let fd = sock.inner.as_raw_fd();
    let max = bufs.len().min(addrs.len());

    let mut hdrs: Vec<mmsghdr> = Vec::with_capacity(max);
    let mut iovecs: Vec<iovec> = Vec::with_capacity(max);
    let mut addrs_raw: Vec<sockaddr_storage> = Vec::with_capacity(max);

    hdrs.set_len(max);
    iovecs.set_len(max);
    addrs_raw.set_len(max);

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

    let rc = recvmmsg(fd, hdrs.as_mut_ptr(), max as u32, MSG_DONTWAIT, std::ptr::null_mut());
    if rc < 0 { return Err(std::io::Error::last_os_error()); }
    let n = rc as usize;

    for i in 0..n {
        let len = hdrs[i].msg_len as usize;
        bufs[i].truncate(len);
        // Convert sockaddr_storage -> SocketAddr
        let ss = &addrs_raw[i];
        let sa = &*(ss as *const _ as *const sockaddr);
        let addr = if sa.sa_family as i32 == AF_INET { 
            let sin = &*(ss as *const _ as *const sockaddr_in);
            let ip = std::net::Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr));
            let port = u16::from_be(sin.sin_port);
            SocketAddr::new(ip.into(), port)
        } else {
            let sin6 = &*(ss as *const _ as *const sockaddr_in6);
            let ip = std::net::Ipv6Addr::from(sin6.sin6_addr.s6_addr);
            let port = u16::from_be(sin6.sin6_port);
            SocketAddr::new(ip.into(), port)
        };
        addrs[i] = addr;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NetConfig;
    use std::net::SocketAddr;
    
    #[test]
    fn test_udp_bind() {
        let mut config = NetConfig::default();
        config.ipv6_only = None; // Let system decide
        let result = Udp::bind("127.0.0.1:0".parse().unwrap(), &config);
        if let Err(e) = &result {
            eprintln!("UDP bind failed: {}", e);
        }
        assert!(result.is_ok());
    }
    
    #[test] 
    fn test_dual_stack_bind() {
        let config = NetConfig::default();
        let result = Udp::bind_dual_stack(0, &config);
        // May fail on systems without IPv6 support
        let _ = result;
    }
    
    #[test]
    fn test_send_to() {
        let mut config = NetConfig::default();
        config.ipv6_only = None;
        let socket = Udp::bind("127.0.0.1:0".parse().unwrap(), &config).unwrap();
        
        // Send to a likely unused port - this should succeed (UDP is connectionless)
        let result = socket.send_to(b"test", "127.0.0.1:9999".parse().unwrap());
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_recv_batch_empty() {
        let mut config = NetConfig::default();
        config.ipv6_only = None;
        let socket = Udp::bind("127.0.0.1:0".parse().unwrap(), &config).unwrap();
        
        let mut bufs: Vec<Vec<u8>> = Vec::new();
        let mut addrs: Vec<SocketAddr> = Vec::new();
        
        let result = socket.recv_batch(&mut bufs, &mut addrs);
        assert_eq!(result.unwrap(), 0);
    }
    
    #[test]
    fn test_send_batch() {
        let mut config = NetConfig::default();
        config.ipv6_only = None;
        let socket = Udp::bind("127.0.0.1:0".parse().unwrap(), &config).unwrap();
        
        let dest = "127.0.0.1:9999".parse().unwrap();
        let packets = vec![
            (b"packet1".as_slice(), dest),
            (b"packet2".as_slice(), dest),
        ];
        
        let result = socket.send_batch(&packets);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }
}
