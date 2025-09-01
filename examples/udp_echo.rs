/*use lowlat-net::{NetConfig, udp::Udp};
use std::net::SocketAddr;


fn main() -> anyhow::Result<()> {
let cfg = NetConfig { busy_poll: Some(50), ..Default::default() };
let addr: SocketAddr = "0.0.0.0:9000".parse()?;
let udp = Udp::bind(addr, &cfg)?;


let mut bufs: Vec<Vec<u8>> = (0..64).map(|_| vec![0u8; 2048]).collect();
let mut addrs: Vec<SocketAddr> = vec![addr; 64];
loop {
let n = udp.recv_batch(&mut bufs[..], &mut addrs[..])?;
for i in 0..n { let _ = udp.send_to(&bufs[i], addrs[i]); }
}
}
*/