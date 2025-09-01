- Add to Cargo.toml: `lowlat-net = { path = "../lowlat-net" }`
- Default backend is mio (epoll/kqueue/IOCP). For io_uring/IOCP via monoio enable:
`lowlat-net = { path = "../lowlat-net", features = ["monoio-runtime"] }`


- Key latency knobs enabled by `NetConfig`:
- TCP_NODELAY, optional TCP_QUICKACK (Linux)
- SO_BUSY_POLL (Linux), SO_REUSEPORT (Linux)
- Tunable recv/send buffers, DSCP/TOS
- Use `Udp::recv_batch` for multi-packet pulls (Linux uses `recvmmsg`).
- Pin your networking threads and isolate cores (suggestion below).
*/


// src/affinity.rs (optional: pin the current thread for jitter reduction)
/*
#[cfg(unix)]
pub fn pin_to_cpu(cpu: usize) { unsafe {
use libc::{cpu_set_t, CPU_SET, CPU_ZERO, sched_setaffinity};
let mut set: cpu_set_t = std::mem::zeroed();
CPU_ZERO(&mut set); CPU_SET(cpu, &mut set);
let _ = sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &set);
}}
#[cfg(windows)]
pub fn pin_to_cpu(cpu: usize) {
use windows_sys::Win32::System::Threading::{SetThreadAffinityMask, GetCurrentThread};
unsafe { let _ = SetThreadAffinityMask(GetCurrentThread(), 1usize << cpu); }
}