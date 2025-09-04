//! Mio-based runtime implementation
//!
//! This module provides a high-performance networking runtime based on the `mio`
//! library, which provides cross-platform async I/O using the best available
//! platform-specific mechanisms:
//!
//! - **Linux**: epoll
//! - **Windows**: IOCP (I/O Completion Ports)
//! - **macOS**: kqueue
//! - **BSD**: kqueue
//!
//! The runtime is designed for high-performance networking applications that
//! require precise control over event handling and minimal overhead.

use mio::net::{
    TcpListener as MioTcpListener, TcpStream as MioTcpStream, UdpSocket as MioUdpSocket,
};
use mio::{Events, Interest, Poll, Token};
use std::{io, time::Duration};

/// High-performance networking runtime using mio
///
/// This runtime provides efficient event-driven networking using the best
/// available I/O mechanism for each platform. It supports configurable
/// polling timeouts and event batch processing for optimal performance.
#[derive(Debug)]
pub struct Runtime {
    /// Core mio poll instance for event notification
    poll: Poll,
    /// Event buffer for batch processing
    events: Events,
    /// Configurable timeout for poll operations
    poll_timeout: Duration,
}

/// Handle for per-socket operations and metadata
///
/// This handle will be expanded in future versions to provide
/// per-socket statistics, configuration, and advanced features.
#[derive(Debug, Clone, Copy)]
pub struct NetHandle;

impl Runtime {
    /// Creates a new runtime with default configuration
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            poll: Poll::new()?,
            events: Events::with_capacity(4096),
            poll_timeout: Duration::from_millis(10),
        })
    }

    /// Creates a runtime with custom event capacity
    pub fn with_capacity(event_capacity: usize) -> io::Result<Self> {
        Ok(Self {
            poll: Poll::new()?,
            events: Events::with_capacity(event_capacity),
            poll_timeout: Duration::from_millis(10),
        })
    }

    /// Sets the polling timeout for event operations
    pub fn set_poll_timeout(&mut self, timeout: Duration) {
        self.poll_timeout = timeout;
    }

    /// Gets the current polling timeout
    pub fn poll_timeout(&self) -> Duration {
        self.poll_timeout
    }

    /// Runs the event loop indefinitely with configurable event handling
    pub fn run<F: FnMut(&mio::event::Event)>(&mut self, mut f: F) -> io::Result<()> {
        loop {
            self.poll.poll(&mut self.events, Some(self.poll_timeout))?;
            for ev in self.events.iter() {
                f(ev);
            }
        }
    }

<<<<<<< HEAD
    pub fn register_udp(&self, socket: &mut MioUdpSocket, token: Token, interest: Interest) -> io::Result<()> { self.poll.registry().register(socket, token, interest) }
    pub fn register_tcp_listener(&self, l: &mut MioTcpListener, token: Token) -> io::Result<()> { self.poll.registry().register(l, token, Interest::READABLE) }
    pub fn register_tcp_stream(&self, s: &mut MioTcpStream, token: Token, interest: Interest) -> io::Result<()> { self.poll.registry().register(s, token, interest) }
}
=======
    /// Runs the event loop with a custom timeout per iteration
    pub fn run_with_timeout<F: FnMut(&mio::event::Event)>(
        &mut self,
        timeout: Duration,
        mut f: F,
    ) -> io::Result<()> {
        loop {
            self.poll.poll(&mut self.events, Some(timeout))?;
            for ev in self.events.iter() {
                f(ev);
            }
        }
    }

    /// Processes events for a single poll cycle
    pub fn poll_once<F: FnMut(&mio::event::Event)>(&mut self, mut f: F) -> io::Result<usize> {
        self.poll.poll(&mut self.events, Some(self.poll_timeout))?;
        let count = self.events.iter().count();
        for ev in self.events.iter() {
            f(ev);
        }
        Ok(count)
    }

    /// Registers a UDP socket for event notification
    pub fn register_udp(
        &self,
        socket: &mut MioUdpSocket,
        token: Token,
        interest: Interest,
    ) -> io::Result<NetHandle> {
        self.poll.registry().register(socket, token, interest)?;
        Ok(NetHandle)
    }

    /// Registers a TCP listener for connection events
    pub fn register_tcp_listener(
        &self,
        listener: &mut MioTcpListener,
        token: Token,
    ) -> io::Result<NetHandle> {
        self.poll
            .registry()
            .register(listener, token, Interest::READABLE)?;
        Ok(NetHandle)
    }

    /// Registers a TCP stream for I/O events
    pub fn register_tcp_stream(
        &self,
        stream: &mut MioTcpStream,
        token: Token,
        interest: Interest,
    ) -> io::Result<NetHandle> {
        self.poll.registry().register(stream, token, interest)?;
        Ok(NetHandle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mio::net::UdpSocket;

    #[test]
    fn test_runtime_creation() {
        let runtime = Runtime::new();
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_runtime_with_capacity() {
        let runtime = Runtime::with_capacity(1024);
        assert!(runtime.is_ok());
    }

    #[test]
    fn test_poll_timeout_configuration() {
        let mut runtime = Runtime::new().unwrap();
        let timeout = Duration::from_millis(5);

        runtime.set_poll_timeout(timeout);
        assert_eq!(runtime.poll_timeout(), timeout);
    }

    #[test]
    fn test_udp_registration() {
        let runtime = Runtime::new().unwrap();
        let mut socket = UdpSocket::bind("127.0.0.1:0".parse().unwrap()).unwrap();

        let result = runtime.register_udp(&mut socket, Token(0), Interest::READABLE);
        assert!(result.is_ok());
    }
}
>>>>>>> origin/main
