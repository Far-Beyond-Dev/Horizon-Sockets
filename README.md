# Horizon-Sockets

A high-performance, low-latency networking library for Rust designed for ultra-fast socket operations with configurable runtime backends. Features comprehensive documentation, extensive performance optimizations, and cross-platform support.

## Features

- **Multiple Runtime Backends**: Choose between `mio` (default) or `monoio` for optimal performance on your platform
- **Cross-Platform**: Full support for Linux, Windows, macOS, BSD, and other Unix-like systems
- **Low-Latency Optimizations**: Built-in support for TCP_NODELAY, SO_BUSY_POLL, TCP_QUICKACK and other latency-reduction techniques
- **Batch Operations**: High-performance batch UDP operations with `recvmmsg` on Linux
- **Buffer Pool Management**: Efficient memory management with reusable buffer pools
- **CPU Affinity Control**: Thread pinning utilities for consistent performance
- **Comprehensive Documentation**: Extensive inline documentation with examples and performance notes
- **Configurable**: Extensive tuning options through `NetConfig` with preset configurations

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
Horizon-Socket-Low = { path = "path/to/Horizon-Sockets" }

# For io_uring backend (Linux) or enhanced IOCP (Windows)
# horizon_sockets = { path = "path/to/Horizon-Sockets", features = ["monoio-runtime"] }
```

### Basic UDP Server

```rust
use horizon_sockets::{NetConfig, udp::Udp};
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
use horizon_sockets::{NetConfig, tcp::TcpListener};

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

The `NetConfig` struct provides extensive tuning options with preset configurations for different use cases:

```rust
use horizon_sockets::NetConfig;

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

### Preset Configurations

The library provides several preset configurations optimized for different scenarios:

#### Default Configuration (Balanced)
```rust
NetConfig::default() // Balanced performance with 4MB buffers
```

#### Low-Latency Configuration
```rust
NetConfig::low_latency() // Optimized for minimal latency
// Features: 50μs busy polling, 256KB buffers, 1ms timeout
```

#### High-Throughput Configuration
```rust
NetConfig::high_throughput() // Optimized for maximum throughput
// Features: 16MB buffers, disabled Nagle's algorithm, 2048 backlog
```

#### Power-Efficient Configuration
```rust
NetConfig::power_efficient() // Optimized for low CPU usage
// Features: 512KB buffers, 100ms timeout, minimal optimizations
```

## Runtime Backends

### Mio Runtime (Default)

Uses `mio` for cross-platform async I/O:
- Linux: epoll
- Windows: IOCP
- macOS/BSD: kqueue

```rust
use horizon_sockets::rt::Runtime;
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

The library includes built-in CPU affinity utilities for consistent performance:

```rust
use horizon_sockets::affinity::{pin_to_cpu, pin_to_cpus, get_cpu_count, get_numa_topology};

// Pin thread to a specific CPU core
pin_to_cpu(2)?; // Pin to CPU core 2

// Pin thread to multiple CPU cores
pin_to_cpus(&[2, 3, 4, 5])?; // Pin to cores 2-5

// Get system information
let cpu_count = get_cpu_count();
let numa_topology = get_numa_topology();
println!("System has {} CPUs across {} NUMA nodes", cpu_count, numa_topology.len());
```

### Buffer Management

The library includes a high-performance buffer pool for efficient memory management:

```rust
use horizon_sockets::buffer_pool::BufferPool;

// Create a buffer pool with 64 buffers of 2KB each
let pool = BufferPool::new(64, 2048);

// Acquire buffers for batch operations
let mut buffers = pool.acquire_batch(32);
let mut addrs = vec![SocketAddr::from(([0,0,0,0], 0)); 32];

loop {
    let count = socket.recv_batch(&mut buffers, &mut addrs)?;
    // Process packets...
    
    // Return buffers to pool for reuse
    pool.release_batch(buffers);
    buffers = pool.acquire_batch(32);
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

- **`config`**: Network configuration and performance tuning parameters with preset configurations
- **`raw`**: Low-level socket operations and platform abstractions for Unix and Windows
- **`udp`**: High-level UDP socket interface with batch operations and comprehensive documentation
- **`tcp`**: High-level TCP socket interface with low-latency optimizations
- **`buffer_pool`**: Memory-efficient buffer pool for network operations with batch management
- **`affinity`**: CPU affinity and thread pinning utilities with NUMA topology detection
- **`rt_mio`**: Mio-based runtime implementation using epoll/kqueue/IOCP
- **`rt_monoio`**: Monoio-based runtime implementation using io_uring/IOCP (under development)

### Platform Support

| Platform | Backend | Special Features |
|----------|---------|------------------|
| Linux | epoll/io_uring | SO_BUSY_POLL, TCP_QUICKACK, recvmmsg |
| Windows | IOCP | WSA overlapped I/O |
| macOS | kqueue | Standard BSD sockets |
| FreeBSD | kqueue | Standard BSD sockets |

## Dependencies

- **`mio`**: Cross-platform async I/O (default backend)
- **`monoio`**: io_uring/IOCP backend (optional, under development)
- **`libc`**: Unix system calls and socket operations
- **`windows-sys`**: Windows system APIs and WinSock2
- **`cfg-if`**: Conditional compilation for platform-specific code

### Development Dependencies
- Standard Rust toolchain
- Platform-specific development tools (Linux kernel headers, Windows SDK)

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request

## Documentation

The library features comprehensive documentation with:

- **Module-level documentation** with examples and architecture overviews
- **Function-level documentation** with usage examples and performance notes
- **Platform-specific notes** detailing behavior differences
- **Performance guidelines** and optimization recommendations
- **Safety documentation** for unsafe operations

## Testing

Run the test suite with:

```bash
cargo test
```

For platform-specific tests:
```bash
# Linux-specific tests (recvmmsg, etc.)
cargo test --features linux-tests

# Windows-specific tests
cargo test --features windows-tests
```

## Roadmap

### In Progress
- [x] Comprehensive inline documentation
- [x] CPU affinity utilities with NUMA support
- [x] Buffer pool management
- [x] Preset configuration system

### Planned
- [ ] Complete monoio runtime implementation with io_uring/IOCP
- [ ] Zero-copy send/receive operations using advanced kernel features
- [ ] DPDK integration for userspace networking
- [ ] Advanced buffer pool with NUMA-aware allocation
- [ ] Real-time scheduling support and priority handling
- [ ] Performance monitoring and metrics collection
- [ ] Async/await interface for modern Rust applications