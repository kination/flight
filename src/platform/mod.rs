use std::net::SocketAddr;

use crate::error::FlightError;

pub(crate) mod fallback;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

#[cfg(target_os = "macos")]
pub(crate) mod macos;

#[allow(dead_code)] // iface used by Linux AF_XDP backend
pub struct PlatformConfig {
    pub port: u16,
    pub iface: Option<String>,
    pub skb_mode: bool,
    pub bind_ip: Option<String>,
}

pub trait PlatformBackend: Sized {
    fn bind(config: &PlatformConfig) -> Result<Self, FlightError>;
    fn transmit(&self, dest: &SocketAddr, data: &[u8]) -> Result<usize, FlightError>;
    fn receive(&self, buf: &mut [u8]) -> Result<(SocketAddr, usize), FlightError>;
    fn local_addr(&self) -> Result<SocketAddr, FlightError>;
    fn close(&mut self) -> Result<(), FlightError>;
}

// macOS: Network.framework backend (Phase 4 구현 전까지 fallback 사용)
// TODO: Phase 4 구현 완료 후 아래로 전환:
//   pub type NativeBackend = macos::NetworkFrameworkBackend;
#[cfg(target_os = "macos")]
pub type NativeBackend = fallback::FallbackBackend;

#[cfg(target_os = "linux")]
pub type NativeBackend = linux::AfXdpBackend;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub type NativeBackend = fallback::FallbackBackend;
