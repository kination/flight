use std::net::{SocketAddr, UdpSocket};

use crate::error::FlightError;
use super::{PlatformBackend, PlatformConfig};

pub struct FallbackBackend {
    socket: UdpSocket,
}

impl PlatformBackend for FallbackBackend {
    fn bind(config: &PlatformConfig) -> Result<Self, FlightError> {
        let addr = format!("0.0.0.0:{}", config.port);
        let socket = UdpSocket::bind(&addr)
            .map_err(|e| FlightError::Bind(format!("{addr}: {e}")))?;
        Ok(Self { socket })
    }

    fn transmit(&self, dest: &SocketAddr, data: &[u8]) -> Result<usize, FlightError> {
        self.socket
            .send_to(data, dest)
            .map_err(|e| FlightError::Send(e.to_string()))
    }

    fn receive(&self, buf: &mut [u8]) -> Result<(SocketAddr, usize), FlightError> {
        let (n, from) = self.socket
            .recv_from(buf)
            .map_err(|e| FlightError::Recv(e.to_string()))?;
        Ok((from, n))
    }

    fn local_addr(&self) -> Result<SocketAddr, FlightError> {
        self.socket.local_addr().map_err(FlightError::Io)
    }

    fn close(&mut self) -> Result<(), FlightError> {
        // UdpSocket closes on drop, nothing to do
        Ok(())
    }
}
