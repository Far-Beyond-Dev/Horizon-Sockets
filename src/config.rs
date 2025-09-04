use std::io;
#[cfg(target_os = "linux")] use std::time::Duration;
use crate::raw;


/// Tunables to push latency down. Defaults are conservative.
#[derive(Clone, Debug)]
pub struct NetConfig {
pub tcp_nodelay: bool,
pub tcp_quickack: bool, // Linux only; ignored elsewhere
pub reuse_port: bool, // SO_REUSEPORT if available
pub busy_poll: Option<u32>, // Linux SO_BUSY_POLL microseconds
pub recv_buf: Option<usize>,
pub send_buf: Option<usize>,
pub tos: Option<u32>, // IP_TOS / DSCP

// IPv6-specific
pub ipv6_only: Option<bool>,
pub hop_limit: Option<i32>,
}


impl Default for NetConfig {
    fn default() -> Self {
    Self {
        tcp_nodelay: true,
        tcp_quickack: true,
        reuse_port: true,
        busy_poll: None,
        recv_buf: Some(1<<20), // 1 MiB
        send_buf: Some(1<<20),
        tos: None,
        ipv6_only: None,
        hop_limit: None,
        }
    }
}


/// Apply low-latency knobs to a socket2::Socket before converting to std::net types.
pub fn apply_low_latency(os: raw::OsSocket, domain: raw::Domain, ty: raw::Type, cfg: &NetConfig) -> io::Result<()> {
    use crate::raw as r;

    if let Some(sz) = cfg.recv_buf { r::set_recv_buffer(os, sz as i32)?; }
    if let Some(sz) = cfg.send_buf { r::set_send_buffer(os, sz as i32)?; }

    // Generic DSCP/TOS
    if let Some(tos) = cfg.tos {
        match domain { r::Domain::Ipv4 => r::set_tos_v4(os, tos as i32)?, r::Domain::Ipv6 => r::set_tos_v6(os, tos as i32)?, }
    }

    // IPv6 toggles
    if let r::Domain::Ipv6 = domain {
        if let Some(only) = cfg.ipv6_only { r::set_ipv6_only(os, only)?; }
        if let Some(hops) = cfg.hop_limit { r::set_ipv6_hop_limit(os, hops)?; }
    }

    // Linux specific tweaks
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        if cfg.reuse_port { r::set_reuse_port(os, true)?; }
        if let Some(us) = cfg.busy_poll { let _ = r::set_busy_poll(os, us); }
        if cfg.tcp_quickack && ty == r::Type::Stream { let _ = r::set_tcp_quickack(os, true); }
    }

    // TCP_NODELAY
    if ty == r::Type::Stream && cfg.tcp_nodelay { r::set_tcp_nodelay(os, true)?; }

    Ok(())
}