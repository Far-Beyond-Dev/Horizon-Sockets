# Horizon-Sockets

A high-performance, low-latency networking library for Rust designed for ultra-fast socket operations with configurable runtime backends.

## Features

- **Multiple Runtime Backends**: Choose between `mio` (default) or `monoio` for optimal performance on your platform
- **Cross-Platform**: Full support for Linux, Windows, and other Unix-like systems
- **Low-Latency Optimizations**: Built-in support for TCP_NODELAY, SO_BUSY_POLL, and other latency-reduction techniques
- **Batch Operations**: High-performance batch UDP operations with `recvmmsg` on Linux
- **Zero-Copy**: Minimal allocations with efficient buffer management
- **Configurable**: Extensive tuning options through `NetConfig`

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
Horizon-Socket-Low = { path = "path/to/Horizon-Sockets" }

# For io_uring backend (Linux) or enhanced IOCP (Windows)
# Horizon-Socket-Low = { path = "path/to/Horizon-Sockets", features = ["monoio-runtime"] }
```

### Basic UDP Server

```rust
use Horizon_Socket_Low::{NetConfig, udp::Udp};
use std::net::SocketAddr;

fn main() -> std::io::Result<()> {
    let config = NetConfig::default();
    let socket = Udp::bind("0.0.0.0:8080".parse().unwrap(), &config)?;
    
    let mut bufs = vec![vec![0u8; 2048]; 32];
    let mut addrs = vec!["0.0.0.0:0".parse().unwrap(); 32];
    
    loop {
        match socket.recv_batch(&mut bufs, &mut addrs) {
            Ok(count) => {
                for i in 0..count {
                    // Echo back the received data
                    socket.send_to(&bufs[i], addrs[i])?;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e),
        }
    }
}
```

### Basic TCP Server

```rust
use Horizon_Socket_Low::{NetConfig, tcp::TcpListener};

fn main() -> std::io::Result<()> {
    let config = NetConfig::default();
    let listener = TcpListener::bind("0.0.0.0:8080".parse().unwrap(), &config)?;
    
    loop {
        match listener.accept_nonblocking() {
            Ok((stream, addr)) => {
                println!("New connection from: {}", addr);
                // Handle connection...
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e),
        }
    }
}
```

## Configuration

### NetConfig Options

The `NetConfig` struct provides extensive tuning options:

```rust
use Horizon_Socket_Low::NetConfig;

let config = NetConfig {
    // TCP optimizations
    tcp_nodelay: true,        // Disable Nagle's algorithm
    tcp_quickack: true,       // Linux: Enable TCP quickack
    
    // Socket options
    reuse_port: true,         // SO_REUSEPORT for load balancing
    recv_buf: Some(4 << 20),  // 4MB receive buffer
    send_buf: Some(4 << 20),  // 4MB send buffer
    
    // Low-latency options
    busy_poll: Some(50),      // Linux: SO_BUSY_POLL in microseconds
    tos: Some(0x10),          // DSCP/TOS marking
    
    // IPv6 settings
    ipv6_only: Some(false),   // Enable dual-stack
    hop_limit: None,          // IPv6 hop limit
};
```

### Default Configuration

The default configuration is optimized for low latency:

```rust
NetConfig {
    tcp_nodelay: true,
    tcp_quickack: true,
    reuse_port: true,
    busy_poll: None,
    recv_buf: Some(1 << 20),  // 1MB
    send_buf: Some(1 << 20),  // 1MB
    tos: None,
    ipv6_only: None,
    hop_limit: None,
}
```

## Runtime Backends

### Mio Runtime (Default)

Uses `mio` for cross-platform async I/O:
- Linux: epoll
- Windows: IOCP
- macOS/BSD: kqueue

```rust
use Horizon_Socket_Low::rt::Runtime;
use mio::{Token, Interest};

let mut runtime = Runtime::new()?;

// Register UDP socket
runtime.register_udp(&mio_socket, Token(0), Interest::READABLE)?;

// Event loop
runtime.run(|event| {
    match event.token() {
        Token(0) => {
            // Handle UDP events
        }
        _ => {}
    }
})?;
```

### Monoio Runtime

Enable with `features = ["monoio-runtime"]` for:
- Linux: io_uring
- Windows: Enhanced IOCP

*Note: Monoio runtime implementation is currently minimal and under development.*

## Advanced Usage

### Batch UDP Operations

For high-throughput UDP applications, use batch operations:

```rust
let socket = Udp::bind(addr, &config)?;

// Prepare buffers for batch receive
let mut bufs: Vec<Vec<u8>> = (0..64)
    .map(|_| Vec::with_capacity(2048))
    .collect();
let mut addrs = vec![SocketAddr::from(([0, 0, 0, 0], 0)); 64];

loop {
    match socket.recv_batch(&mut bufs, &mut addrs) {
        Ok(count) => {
            // Process 'count' received packets
            for i in 0..count {
                process_packet(&bufs[i], addrs[i]);
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            // No packets available, continue or yield
            continue;
        }
        Err(e) => return Err(e),
    }
}
```

### Dual-Stack IPv6 Binding

```rust
// Bind to IPv6 with IPv4 compatibility
let socket = Udp::bind_dual_stack(8080, &config)?;
```

### Platform-Specific Optimizations

#### Linux Optimizations

```rust
let config = NetConfig {
    tcp_quickack: true,       // Reduce ACK delay
    reuse_port: true,         // Load balance across threads
    busy_poll: Some(50),      // Poll network device for 50μs
    ..Default::default()
};
```

#### Windows Optimizations

```rust
let config = NetConfig {
    recv_buf: Some(8 << 20),  // Larger buffers for IOCP
    send_buf: Some(8 << 20),
    ..Default::default()
};
```

## Performance Tips

### CPU Affinity

Pin network threads to specific CPU cores for consistent performance:

```rust
#[cfg(unix)]
fn pin_to_cpu(cpu: usize) -> std::io::Result<()> {
    use libc::{cpu_set_t, CPU_SET, CPU_ZERO, sched_setaffinity};
    
    unsafe {
        let mut set: cpu_set_t = std::mem::zeroed();
        CPU_ZERO(&mut set);
        CPU_SET(cpu, &mut set);
        
        if sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &set) != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(windows)]
fn pin_to_cpu(cpu: usize) -> std::io::Result<()> {
    use windows_sys::Win32::System::Threading::{SetThreadAffinityMask, GetCurrentThread};
    
    unsafe {
        if SetThreadAffinityMask(GetCurrentThread(), 1usize << cpu) == 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

// Usage
pin_to_cpu(2)?; // Pin to CPU core 2
```

### Buffer Management

Pre-allocate buffers to avoid runtime allocations:

```rust
// Pre-allocate buffer pool
let mut buffer_pool: Vec<Vec<u8>> = (0..128)
    .map(|_| vec![0u8; 2048])
    .collect();

// Reuse buffers across recv operations
loop {
    let count = socket.recv_batch(&mut buffer_pool[..32], &mut addrs)?;
    // Process packets...
    
    // Buffers are automatically reused
}
```

### Tuning Guidelines

1. **Buffer Sizes**: Start with 1-4MB buffers, increase for high-throughput applications
2. **Busy Polling**: Use 10-100μs on dedicated cores, disable on shared systems
3. **Batch Size**: Use 16-64 packet batches for UDP applications
4. **Thread Count**: Typically 1 thread per CPU core for network-intensive workloads

## Error Handling

Common error patterns and handling:

```rust
match socket.recv_batch(&mut bufs, &mut addrs) {
    Ok(count) => {
        // Success: processed 'count' packets
    }
    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
        // No data available, continue polling
    }
    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
        // System call interrupted, retry
    }
    Err(e) => {
        // Actual error occurred
        eprintln!("Network error: {}", e);
        return Err(e);
    }
}
```

## Architecture

### Module Structure

- `config`: Network configuration and tuning parameters
- `raw`: Low-level socket operations and platform abstractions
- `udp`: High-level UDP socket interface with batch operations
- `tcp`: High-level TCP socket interface
- `rt_mio`: Mio-based runtime implementation
- `rt_monoio`: Monoio-based runtime implementation (minimal)

### Platform Support

| Platform | Backend | Special Features |
|----------|---------|------------------|
| Linux | epoll/io_uring | SO_BUSY_POLL, TCP_QUICKACK, recvmmsg |
| Windows | IOCP | WSA overlapped I/O |
| macOS | kqueue | Standard BSD sockets |
| FreeBSD | kqueue | Standard BSD sockets |

## Dependencies

- `mio`: Cross-platform async I/O (default backend)
- `monoio`: io_uring/IOCP backend (optional)
- `libc`: Unix system calls
- `windows-sys`: Windows system APIs
- `bytemuck`: Safe transmutation
- `cfg-if`: Conditional compilation
- `slab`: Token allocation (mio backend)

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request

## Roadmap

- [ ] Complete monoio runtime implementation
- [ ] Zero-copy send/receive operations
- [ ] DPDK integration for userspace networking
- [ ] Advanced buffer pool management
- [ ] Automatic NUMA topology detection
- [ ] Real-time scheduling support
- [ ] Performance monitoring and metrics