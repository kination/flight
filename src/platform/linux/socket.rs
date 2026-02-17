use std::os::unix::io::RawFd;

use crate::error::FlightError;
use super::consts::*;
use super::rings::*;
use super::umem::Umem;

/// AF_XDP 소켓 래퍼. 소켓 생성, UMEM 등록, ring 매핑, 인터페이스 바인딩을 담당한다.
pub struct XskSocket {
    pub(crate) fd: RawFd,
}

impl XskSocket {
    pub fn new() -> Result<Self, FlightError> {
        let fd = unsafe { libc::socket(AF_XDP, libc::SOCK_RAW, 0) };
        if fd < 0 {
            return Err(FlightError::Bind(format!(
                "AF_XDP socket creation failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(Self { fd })
    }

    /// UMEM을 소켓에 등록한다.
    pub fn register_umem(&self, umem: &Umem) -> Result<(), FlightError> {
        let reg = XdpUmemReg {
            addr: umem.ptr as u64,
            len: umem.size as u64,
            chunk_size: umem.frame_size as u32,
            headroom: 0,
            flags: 0,
        };

        let ret = unsafe {
            libc::setsockopt(
                self.fd,
                SOL_XDP,
                XDP_UMEM_REG,
                &reg as *const _ as *const libc::c_void,
                std::mem::size_of::<XdpUmemReg>() as libc::socklen_t,
            )
        };
        if ret < 0 {
            return Err(FlightError::Bind(format!(
                "UMEM registration failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(())
    }

    /// Fill/Completion/RX/TX ring 크기를 설정한다.
    pub fn set_ring_sizes(
        &self,
        fill: u32,
        comp: u32,
        rx: u32,
        tx: u32,
    ) -> Result<(), FlightError> {
        for (opt, val, name) in [
            (XDP_UMEM_FILL_RING, fill, "fill"),
            (XDP_UMEM_COMPLETION_RING, comp, "completion"),
            (XDP_RX_RING, rx, "rx"),
            (XDP_TX_RING, tx, "tx"),
        ] {
            let ret = unsafe {
                libc::setsockopt(
                    self.fd,
                    SOL_XDP,
                    opt,
                    &val as *const _ as *const libc::c_void,
                    std::mem::size_of::<u32>() as libc::socklen_t,
                )
            };
            if ret < 0 {
                return Err(FlightError::Bind(format!(
                    "set {name} ring size failed: {}",
                    std::io::Error::last_os_error()
                )));
            }
        }
        Ok(())
    }

    /// getsockopt으로 ring mmap offset 정보를 가져온다.
    pub fn get_mmap_offsets(&self) -> Result<XdpMmapOffsets, FlightError> {
        let mut offsets = XdpMmapOffsets::default();
        let mut len = std::mem::size_of::<XdpMmapOffsets>() as libc::socklen_t;

        let ret = unsafe {
            libc::getsockopt(
                self.fd,
                SOL_XDP,
                XDP_MMAP_OFFSETS,
                &mut offsets as *mut _ as *mut libc::c_void,
                &mut len,
            )
        };
        if ret < 0 {
            return Err(FlightError::Bind(format!(
                "get mmap offsets failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(offsets)
    }

    /// ring별 mmap을 수행하고 Ring 구조체를 반환한다.
    pub fn mmap_rings(
        &self,
        offsets: &XdpMmapOffsets,
        ring_size: u32,
    ) -> Result<(FillRing, CompletionRing, RxRing, TxRing), FlightError> {
        let desc_size_u64 = ring_size as usize * std::mem::size_of::<u64>();
        let desc_size_xdp = ring_size as usize * std::mem::size_of::<XdpDesc>();

        // Fill ring mmap
        let fill_mmap_size = offsets.fr.desc as usize + desc_size_u64;
        let fill_base = self.mmap_ring(fill_mmap_size, XDP_UMEM_PGOFF_FILL_RING)?;
        let fill_ring = unsafe { FillRing::new(fill_base, &offsets.fr, ring_size) };

        // Completion ring mmap
        let comp_mmap_size = offsets.cr.desc as usize + desc_size_u64;
        let comp_base = self.mmap_ring(comp_mmap_size, XDP_UMEM_PGOFF_COMPLETION_RING)?;
        let comp_ring = unsafe { CompletionRing::new(comp_base, &offsets.cr, ring_size) };

        // RX ring mmap
        let rx_mmap_size = offsets.rx.desc as usize + desc_size_xdp;
        let rx_base = self.mmap_ring(rx_mmap_size, XDP_PGOFF_RX_RING)?;
        let rx_ring = unsafe { RxRing::new(rx_base, &offsets.rx, ring_size) };

        // TX ring mmap
        let tx_mmap_size = offsets.tx.desc as usize + desc_size_xdp;
        let tx_base = self.mmap_ring(tx_mmap_size, XDP_PGOFF_TX_RING)?;
        let tx_ring = unsafe { TxRing::new(tx_base, &offsets.tx, ring_size) };

        Ok((fill_ring, comp_ring, rx_ring, tx_ring))
    }

    fn mmap_ring(&self, size: usize, pgoff: i64) -> Result<*mut u8, FlightError> {
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_POPULATE,
                self.fd,
                pgoff,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(FlightError::Bind(format!(
                "ring mmap failed (offset={pgoff:#x}): {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(ptr as *mut u8)
    }

    /// AF_XDP 소켓을 네트워크 인터페이스에 바인딩한다.
    pub fn bind_to_iface(
        &self,
        ifindex: u32,
        queue_id: u32,
        flags: u16,
    ) -> Result<(), FlightError> {
        let addr = SockaddrXdp {
            sxdp_family: AF_XDP as u16,
            sxdp_flags: flags,
            sxdp_ifindex: ifindex,
            sxdp_queue_id: queue_id,
            sxdp_shared_umem_fd: 0,
        };

        let ret = unsafe {
            libc::bind(
                self.fd,
                &addr as *const _ as *const libc::sockaddr,
                std::mem::size_of::<SockaddrXdp>() as libc::socklen_t,
            )
        };
        if ret < 0 {
            return Err(FlightError::Bind(format!(
                "AF_XDP bind failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(())
    }

    /// TX ring에 데이터를 넣은 후 커널에 전송을 알린다.
    pub fn kick_tx(&self) -> Result<(), FlightError> {
        let ret = unsafe {
            libc::sendto(
                self.fd,
                std::ptr::null(),
                0,
                libc::MSG_DONTWAIT,
                std::ptr::null(),
                0,
            )
        };
        // EAGAIN/ENOBUFS는 정상 (커널이 이미 처리 중)
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            let errno = err.raw_os_error().unwrap_or(0);
            if errno != libc::EAGAIN && errno != libc::ENOBUFS && errno != libc::EBUSY {
                return Err(FlightError::Send(format!("TX kick failed: {err}")));
            }
        }
        Ok(())
    }

    /// poll()로 수신 대기한다.
    pub fn poll_rx(&self, timeout_ms: i32) -> Result<bool, FlightError> {
        let mut pfd = libc::pollfd {
            fd: self.fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
        if ret < 0 {
            return Err(FlightError::Recv(format!(
                "poll failed: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(ret > 0 && (pfd.revents & libc::POLLIN) != 0)
    }
}

impl Drop for XskSocket {
    fn drop(&mut self) {
        if self.fd >= 0 {
            unsafe { libc::close(self.fd) };
        }
    }
}
