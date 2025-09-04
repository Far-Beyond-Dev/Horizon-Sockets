use std::{io, time::Duration};
use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpListener as MioTcpListener, TcpStream as MioTcpStream, UdpSocket as MioUdpSocket};
use slab::Slab;

pub struct Runtime {
    poll: Poll,
    events: Events,
}

pub struct NetHandle; // placeholder for future per-socket handles

impl Runtime {
    pub fn new() -> io::Result<Self> { Ok(Self { poll: Poll::new()?, events: Events::with_capacity(4096) }) }

    /// Basic single-threaded poll loop. Provide a closure that handles events.
    pub fn run<F: FnMut(&mio::event::Event)>(&mut self, mut f: F) -> io::Result<()> {
        loop {
            self.poll.poll(&mut self.events, Some(Duration::from_millis(10)))?;
            for ev in self.events.iter() { f(ev); }
        }
    }

    pub fn register_udp(&self, socket: &mut MioUdpSocket, token: Token, interest: Interest) -> io::Result<()> { self.poll.registry().register(socket, token, interest) }
    pub fn register_tcp_listener(&self, l: &mut MioTcpListener, token: Token) -> io::Result<()> { self.poll.registry().register(l, token, Interest::READABLE) }
    pub fn register_tcp_stream(&self, s: &mut MioTcpStream, token: Token, interest: Interest) -> io::Result<()> { self.poll.registry().register(s, token, interest) }
}