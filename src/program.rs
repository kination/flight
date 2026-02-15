use std::net::SocketAddr;
use std::sync::atomic::Ordering;

use crate::context::{Action, Context, Mode, Rule};
use crate::error::FlightError;
use crate::platform::{NativeBackend, PlatformBackend};
use crate::stats::Stats;


pub struct Program {
    context: Context,
    backend: Option<NativeBackend>,
    stats: Stats,
}

impl Program {
    pub fn with_context(ctx: Context) -> Result<Self, FlightError> {
        Ok(Self {
            context: ctx,
            backend: None,
            stats: Stats::new(),
        })
    }

    pub fn attach(&mut self) -> Result<(), FlightError> {
        let config = self.context.to_platform_config();
        let backend = NativeBackend::bind(&config)?;

        let local = backend.local_addr()?;
        println!("[flight] attached on {local} (mode: {:?})", self.context.mode);

        self.backend = Some(backend);
        Ok(())
    }

    pub fn send(&self, dest: &SocketAddr, data: &[u8]) -> Result<usize, FlightError> {
        let backend = self.backend.as_ref().ok_or(FlightError::NotAttached)?;

        if self.should_drop_outgoing(dest) {
            self.stats.dropped.fetch_add(1, Ordering::Relaxed);
            return Ok(0);
        }

        let n = backend.transmit(dest, data)?;

        self.stats.tx_packets.fetch_add(1, Ordering::Relaxed);
        self.stats.tx_bytes.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }

    pub fn recv(&self, buf: &mut [u8]) -> Result<(SocketAddr, usize), FlightError> {
        let backend = self.backend.as_ref().ok_or(FlightError::NotAttached)?;

        let (from, n) = backend.receive(buf)?;

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
        self.backend
            .as_ref()
            .ok_or(FlightError::NotAttached)?
            .local_addr()
    }

    pub fn detach(&mut self) -> Result<(), FlightError> {
        if let Some(mut backend) = self.backend.take() {
            let addr = backend.local_addr()?;
            backend.close()?;
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
