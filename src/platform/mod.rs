use std::net::SocketAddr;

use crate::error::FlightError;

pub(crate) mod fallback;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

#[allow(dead_code)] // iface used by Linux AF_XDP backend
pub struct PlatformConfig {
    pub port: u16,
    pub iface: Option<String>,
    pub skb_mode: bool,
}

pub trait PlatformBackend: Sized {
    fn bind(config: &PlatformConfig) -> Result<Self, FlightError>;
    fn transmit(&self, dest: &SocketAddr, data: &[u8]) -> Result<usize, FlightError>;
    fn receive(&self, buf: &mut [u8]) -> Result<(SocketAddr, usize), FlightError>;
    fn local_addr(&self) -> Result<SocketAddr, FlightError>;
    fn close(&mut self) -> Result<(), FlightError>;
}

#[cfg(not(target_os = "linux"))]
pub type NativeBackend = fallback::FallbackBackend;

#[cfg(target_os = "linux")]
pub type NativeBackend = linux::AfXdpBackend;
