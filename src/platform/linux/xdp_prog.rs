use std::os::unix::io::RawFd;

use aya::programs::{Xdp, XdpFlags};
use aya::Ebpf;

use crate::error::FlightError;

/// 컴파일된 eBPF XDP 바이너리 (include_bytes!로 임베드)
#[repr(C, align(8))]
struct AlignedBpf {
    data: [u8; include_bytes!("../../../xpresso-ebpf/target/bpfel-unknown-none/release/xpresso-redirect").len()],
}

static XPRESSO_XDP_BPF_ALIGNED: AlignedBpf = AlignedBpf {
    data: *include_bytes!("../../../xpresso-ebpf/target/bpfel-unknown-none/release/xpresso-redirect"),
};

static XPRESSO_XDP_BPF: &[u8] = &XPRESSO_XDP_BPF_ALIGNED.data;

/// XDP 프로그램 로드/어태치 핸들.
/// Drop 시 자동으로 프로그램이 디태치된다.
pub struct XdpHandle {
    _ebpf: Ebpf,
}

impl XdpHandle {
    /// XDP 프로그램을 로드하고 인터페이스에 어태치한다.
    /// xsk_fd를 XSK_MAP[queue_id]에 등록한다.
    pub fn load_and_attach(
        iface: &str,
        xsk_fd: RawFd,
        queue_id: u32,
        skb_mode: bool,
    ) -> Result<Self, FlightError> {
        let mut ebpf = Ebpf::load(XPRESSO_XDP_BPF)
            .map_err(|e| FlightError::Bind(format!("eBPF load: {e}")))?;

        let program: &mut Xdp = ebpf
            .program_mut("xpresso_redirect")
            .ok_or_else(|| FlightError::Bind("XDP program 'xpresso_redirect' not found".into()))?
            .try_into()
            .map_err(|e| FlightError::Bind(format!("not an XDP program: {e}")))?;

        program
            .load()
            .map_err(|e| FlightError::Bind(format!("XDP load: {e}")))?;

        let flags = if skb_mode {
            XdpFlags::SKB_MODE
        } else {
            XdpFlags::default()
        };

        program
            .attach(iface, flags)
            .map_err(|e| FlightError::Bind(format!("XDP attach to '{iface}': {e}")))?;

        // AF_XDP 소켓 fd를 XSK_MAP에 등록
        let mut xsk_map: aya::maps::XskMap<&mut aya::maps::MapData> = ebpf
            .map_mut("XSK_MAP")
            .ok_or_else(|| FlightError::Bind("XSK_MAP not found in eBPF".into()))?
            .try_into()
            .map_err(|e| FlightError::Bind(format!("XSK_MAP type error: {e}")))?;

        xsk_map
            .set(queue_id, xsk_fd, 0)
            .map_err(|e| FlightError::Bind(format!("XSK_MAP.set: {e}")))?;

        println!("[xpresso] XDP program attached to '{iface}' (queue={queue_id})");

        Ok(Self { _ebpf: ebpf })
    }
}
