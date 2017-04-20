use {UtpSocket, UtpListener, UtpStream};
use mio::*;
use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

pub struct Harness {
    socket: UtpSocket,
    listener: UtpListener,
    poll: Poll,
}

impl Harness {
    pub fn new() -> Harness {
        let (socket, listener) = UtpSocket::bind(&"127.0.0.1:0".parse().unwrap()).unwrap();
        let poll = Poll::new().unwrap();

        // Register the sockets
        poll.register(&socket, Token(0),
                      Ready::readable() | Ready::writable(),
                      PollOpt::edge()).unwrap();

        poll.register(&listener, Token(1),
                     Ready::readable(),
                     PollOpt::edge()).unwrap();

        Harness {
            socket: socket,
            listener: listener,
            poll: poll,
        }
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }

    pub fn connect(&self, remote: SocketAddr) -> UtpStream {
        let stream = self.socket.connect(&remote).unwrap();

        self.poll.register(&stream, Token(2),
                           Ready::readable() | Ready::writable(),
                           PollOpt::edge()).unwrap();

        stream
    }

    pub fn wait<F, T>(&self, f: F) -> io::Result<T>
        where F: FnMut() -> io::Result<T>,
    {
        self.wait_ms(10_000, f)
    }

    pub fn wait_until<F>(&self, mut f: F)
        where F: FnMut() -> bool,
    {
        self.wait(|| {
            if f() {
                Ok(())
            } else {
                Err(io::ErrorKind::WouldBlock.into())
            }
        }).unwrap();
    }

    pub fn wait_ms<F, T>(&self, ms: u64, mut f: F) -> io::Result<T>
        where F: FnMut() -> io::Result<T>,
    {
        loop {
            self.tick();

            match f() {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        return Err(e);
                    }
                }
            }
        }
    }

    pub fn tick(&self) {
        let mut events = Events::with_capacity(4);

        self.poll.poll(&mut events, None).unwrap();

        for e in events.iter() {
            if e.token() == Token(0) {
                self.socket.ready(e.readiness()).unwrap();
            }
        }
    }

    pub fn tick_for(&self, ms: u64) {
        let mut events = Events::with_capacity(4);
        let now = Instant::now();
        let dur = Duration::from_millis(ms);

        loop {
            let elapsed = now.elapsed();

            if elapsed >= dur {
                return;
            }

            self.poll.poll(&mut events, Some(dur - elapsed)).unwrap();

            for e in events.iter() {
                if e.token() == Token(0) {
                    self.socket.ready(e.readiness()).unwrap();
                }
            }
        }
    }
}
