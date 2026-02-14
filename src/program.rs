use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::Ordering;

use crate::context::{Action, Context, Mode, Rule};
use crate::error::FlightError;
use crate::stats::Stats;


pub struct Program {
    context: Context,
    socket: Option<UdpSocket>,
    stats: Stats,
}

impl Program {
    pub fn with_context(ctx: Context) -> Result<Self, FlightError> {
        Ok(Self {
            context: ctx,
            socket: None,
            stats: Stats::new(),
        })
    }

    pub fn attach(&mut self) -> Result<(), FlightError> {
        let addr = format!("0.0.0.0:{}", self.context.port);
        let socket = UdpSocket::bind(&addr)
            .map_err(|e| FlightError::Bind(format!("{addr}: {e}")))?;

        let local = socket.local_addr()?;
        println!("[flight] attached on {local} (mode: {:?})", self.context.mode);

        self.socket = Some(socket);
        Ok(())
    }

    pub fn send(&self, dest: &SocketAddr, data: &[u8]) -> Result<usize, FlightError> {
        let socket = self.socket.as_ref().ok_or(FlightError::NotAttached)?;

        if self.should_drop_outgoing(dest) {
            self.stats.dropped.fetch_add(1, Ordering::Relaxed);
            return Ok(0);
        }

        let n = socket
            .send_to(data, dest)
            .map_err(|e| FlightError::Send(e.to_string()))?;

        self.stats.tx_packets.fetch_add(1, Ordering::Relaxed);
        self.stats.tx_bytes.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }

    pub fn recv(&self, buf: &mut [u8]) -> Result<(SocketAddr, usize), FlightError> {
        let socket = self.socket.as_ref().ok_or(FlightError::NotAttached)?;

        let (n, from) = socket
            .recv_from(buf)
            .map_err(|e| FlightError::Recv(e.to_string()))?;

        if self.should_drop_incoming(&from) {
            self.stats.dropped.fetch_add(1, Ordering::Relaxed);
            return self.recv(buf);
        }

        self.stats.rx_packets.fetch_add(1, Ordering::Relaxed);
        self.stats.rx_bytes.fetch_add(n as u64, Ordering::Relaxed);

        if self.context.mode == Mode::Echo {
            let _ = self.send(&from, &buf[..n]);
        }

        Ok((from, n))
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    pub fn local_addr(&self) -> Result<SocketAddr, FlightError> {
        self.socket
            .as_ref()
            .ok_or(FlightError::NotAttached)?
            .local_addr()
            .map_err(FlightError::Io)
    }

    pub fn detach(&mut self) -> Result<(), FlightError> {
        if let Some(socket) = self.socket.take() {
            let addr = socket.local_addr()?;
            drop(socket);
            println!("[flight] detached from {addr}");
        }
        Ok(())
    }

    fn should_drop_incoming(&self, from: &SocketAddr) -> bool {
        if self.context.default_action == Action::Drop {
            
            return !self.matches_allow_rule(from);
        }

        self.matches_deny_rule(from)
    }

    fn should_drop_outgoing(&self, dest: &SocketAddr) -> bool {
        self.matches_deny_rule(dest)
    }

    fn matches_deny_rule(&self, addr: &SocketAddr) -> bool {
        self.context.rules.iter().any(|rule| match rule {
            Rule::DenyPort(port) => addr.port() == *port,
            _ => false,
        })
    }

    fn matches_allow_rule(&self, addr: &SocketAddr) -> bool {
        self.context.rules.iter().any(|rule| match rule {
            Rule::AllowCidr(cidr) => {
                
                if let Some(prefix) = cidr.split('/').next() {
                    addr.ip().to_string().starts_with(prefix.trim_end_matches(".0"))
                } else {
                    false
                }
            }
            _ => false,
        })
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        let _ = self.detach();
    }
}
