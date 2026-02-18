mod consts;
mod frame_alloc;
pub(crate) mod packet;
mod rings;
mod socket;
mod umem;
mod xdp_prog;

use std::cell::UnsafeCell;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use super::{PlatformBackend, PlatformConfig};
use crate::error::FlightError;

use consts::*;
use frame_alloc::FrameAllocator;
use rings::*;
use socket::XskSocket;
use umem::Umem;
use xdp_prog::XdpHandle;

/// AF_XDP 기반 고성능 백엔드.
/// UMEM 공유 메모리와 4개의 ring buffer를 통해 커널을 바이패스하여 패킷을 송수신한다.
pub struct AfXdpBackend {
    socket: XskSocket,
    umem: Umem,
    /// 내부 가변 상태 (ring buffers, frame allocator)
    /// PlatformBackend trait의 transmit/receive가 &self이므로 UnsafeCell 사용
    inner: UnsafeCell<Inner>,
    _xdp_handle: XdpHandle,
    src_mac: [u8; 6],
    src_ip: Ipv4Addr,
    src_port: u16,
    ifindex: u32,
}

struct Inner {
    fill_ring: FillRing,
    comp_ring: CompletionRing,
    rx_ring: RxRing,
    tx_ring: TxRing,
    frame_alloc: FrameAllocator,
}

impl AfXdpBackend {
    fn inner(&self) -> &mut Inner {
        unsafe { &mut *self.inner.get() }
    }

    /// 인터페이스의 MAC 주소를 읽는다.
    fn read_iface_mac(iface: &str) -> Result<[u8; 6], FlightError> {
        let path = format!("/sys/class/net/{iface}/address");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| FlightError::Bind(format!("read MAC from {path}: {e}")))?;
        let parts: Vec<u8> = content
            .trim()
            .split(':')
            .map(|s| u8::from_str_radix(s, 16).unwrap_or(0))
            .collect();
        if parts.len() != 6 {
            return Err(FlightError::Bind(format!("invalid MAC format: {content}")));
        }
        let mut mac = [0u8; 6];
        mac.copy_from_slice(&parts);
        Ok(mac)
    }

    /// 인터페이스의 IPv4 주소를 읽는다.
    fn read_iface_ip(iface: &str) -> Result<Ipv4Addr, FlightError> {
        // ip -4 addr show <iface> 에서 파싱
        let output = std::process::Command::new("ip")
            .args(["-4", "-o", "addr", "show", iface])
            .output()
            .map_err(|e| FlightError::Bind(format!("ip addr show: {e}")))?;

        let text = String::from_utf8_lossy(&output.stdout);
        // "2: veth0    inet 10.0.0.1/24 ..."
        for word in text.split_whitespace() {
            if let Some(ip_str) = word.split('/').next() {
                if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                    return Ok(ip);
                }
            }
        }
        Err(FlightError::Bind(format!("no IPv4 address on {iface}")))
    }

    /// Completion Ring에서 완료된 프레임을 회수하여 FrameAllocator에 반환한다.
    fn reclaim_completed(&self) {
        let inner = self.inner();
        while let Some(addr) = inner.comp_ring.consume() {
            inner.frame_alloc.free(addr);
        }
    }
}

impl PlatformBackend for AfXdpBackend {
    fn bind(config: &PlatformConfig) -> Result<Self, FlightError> {
        let iface = config.iface.as_deref().unwrap_or("eth0");
        let ring_size = DEFAULT_RING_SIZE;
        let frame_count = DEFAULT_FRAME_COUNT as usize;
        let frame_size = DEFAULT_FRAME_SIZE as usize;

        // 1. 인터페이스 정보
        let ifindex = unsafe {
            libc::if_nametoindex(
                std::ffi::CString::new(iface)
                    .map_err(|e| FlightError::Bind(format!("invalid iface name: {e}")))?
                    .as_ptr(),
            )
        };
        if ifindex == 0 {
            return Err(FlightError::Bind(format!(
                "interface '{iface}' not found: {}",
                std::io::Error::last_os_error()
            )));
        }

        let src_mac = Self::read_iface_mac(iface)?;
        let src_ip = if let Some(ip_str) = config.bind_ip.as_deref() {
            ip_str
                .parse()
                .map_err(|e| FlightError::Bind(format!("invalid bind IP {ip_str}: {e}")))?
        } else {
            Self::read_iface_ip(iface)?
        };

        println!(
            "[xpresso] AF_XDP: iface={iface} ifindex={ifindex} mac={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x} ip={src_ip}",
            src_mac[0], src_mac[1], src_mac[2], src_mac[3], src_mac[4], src_mac[5]
        );

        // 2. UMEM 할당
        let umem = Umem::new(frame_count, frame_size)?;
        println!(
            "[xpresso] AF_XDP: UMEM allocated {}KB ({frame_count} x {frame_size}B frames)",
            umem.size / 1024
        );

        // 3. AF_XDP 소켓 생성
        let socket = XskSocket::new()?;

        // 4. UMEM 등록
        socket.register_umem(&umem)?;

        // 5. Ring 크기 설정
        socket.set_ring_sizes(ring_size, ring_size, ring_size, ring_size)?;

        // 6. Ring mmap
        let offsets = socket.get_mmap_offsets()?;
        let (mut fill_ring, comp_ring, rx_ring, tx_ring) =
            socket.mmap_rings(&offsets, ring_size)?;

        // 7. XDP 프로그램 로드 및 어태치
        // SKB 모드 사용 여부는 config에서 결정
        let xdp_handle = XdpHandle::load_and_attach(iface, socket.fd, 0, config.skb_mode)?;

        // 8. 인터페이스에 바인딩
        socket.bind_to_iface(ifindex, 0, XDP_COPY)?;

        // 9. Fill Ring에 초기 프레임 주소를 채운다
        let mut frame_alloc = FrameAllocator::new(frame_count, frame_size);
        let initial_fill = ring_size as usize;
        let addrs: Vec<u64> = (0..initial_fill)
            .filter_map(|_| frame_alloc.alloc())
            .collect();
        fill_ring.produce_batch(&addrs);
        println!(
            "[xpresso] AF_XDP: fill ring initialized with {} frames",
            addrs.len()
        );

        Ok(Self {
            socket,
            umem,
            inner: UnsafeCell::new(Inner {
                fill_ring,
                comp_ring,
                rx_ring,
                tx_ring,
                frame_alloc,
            }),
            _xdp_handle: xdp_handle,
            src_mac,
            src_ip,
            src_port: config.port,
            ifindex,
        })
    }

    fn transmit(&self, dest: &SocketAddr, data: &[u8]) -> Result<usize, FlightError> {
        let inner = self.inner();

        // 완료된 TX 프레임 회수
        self.reclaim_completed();

        // 프레임 할당
        let frame_addr = inner
            .frame_alloc
            .alloc()
            .ok_or_else(|| FlightError::Send("UMEM full: no free frames".into()))?;

        // 프레임에 Ethernet + IPv4 + UDP 패킷 구성
        let frame_buf = unsafe {
            std::slice::from_raw_parts_mut(self.umem.frame_ptr(frame_addr), self.umem.frame_size)
        };

        let (dst_ip, dst_port) = match dest {
            SocketAddr::V4(v4) => (*v4.ip(), v4.port()),
            SocketAddr::V6(_) => {
                inner.frame_alloc.free(frame_addr);
                return Err(FlightError::Send("IPv6 not supported yet".into()));
            }
        };

        // TODO: dst_mac을 ARP로 해석 (현재는 broadcast)
        let dst_mac = [0xff; 6];

        let frame_len = packet::build_udp_frame(
            self.src_mac,
            dst_mac,
            self.src_ip,
            dst_ip,
            self.src_port,
            dst_port,
            data,
            frame_buf,
        )
        .ok_or_else(|| {
            inner.frame_alloc.free(frame_addr);
            FlightError::Send("payload too large for frame".into())
        })?;

        // TX Ring에 descriptor 추가
        inner.tx_ring.produce(consts::XdpDesc {
            addr: frame_addr,
            len: frame_len as u32,
            options: 0,
        });

        // 커널에 전송 알림
        self.socket.kick_tx()?;

        Ok(data.len())
    }

    fn receive(&self, buf: &mut [u8]) -> Result<(SocketAddr, usize), FlightError> {
        let inner = self.inner();

        loop {
            // RX Ring에서 패킷 확인
            if let Some(desc) = inner.rx_ring.consume() {
                let frame = unsafe {
                    std::slice::from_raw_parts(self.umem.frame_ptr(desc.addr), desc.len as usize)
                };

                // 패킷 파싱
                let result = if let Some((from, payload)) = packet::parse_udp_frame(frame) {
                    let n = payload.len().min(buf.len());
                    buf[..n].copy_from_slice(&payload[..n]);
                    Ok((from, n))
                } else {
                    // 파싱 실패 - 프레임 반환 후 재시도
                    inner.fill_ring.produce(desc.addr);
                    continue;
                };

                // 프레임을 Fill Ring에 반환
                inner.fill_ring.produce(desc.addr);

                return result;
            }

            // 패킷이 없으면 poll로 대기
            self.socket.poll_rx(-1)?;
        }
    }

    fn local_addr(&self) -> Result<SocketAddr, FlightError> {
        Ok(SocketAddr::V4(SocketAddrV4::new(
            self.src_ip,
            self.src_port,
        )))
    }

    fn close(&mut self) -> Result<(), FlightError> {
        println!(
            "[xpresso] AF_XDP: closing socket on ifindex={}",
            self.ifindex
        );
        // XdpHandle, XskSocket, Umem 모두 Drop에서 정리됨
        Ok(())
    }
}

// SAFETY: AfXdpBackend의 내부 가변 상태는 UnsafeCell을 통해 접근하며,
// 현재 단일 스레드에서만 사용된다. 추후 멀티스레드 지원 시 Mutex로 교체.
unsafe impl Send for AfXdpBackend {}
