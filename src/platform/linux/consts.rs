// Linux AF_XDP constants not yet in the libc crate.
// These are stable kernel ABI from <linux/if_xdp.h>.

#![allow(dead_code)]

pub const AF_XDP: i32 = 44;
pub const SOL_XDP: i32 = 283;

// setsockopt options
// From linux/if_xdp.h
pub const XDP_MMAP_OFFSETS: i32 = 1;
pub const XDP_RX_RING: i32 = 2;
pub const XDP_TX_RING: i32 = 3;
pub const XDP_UMEM_REG: i32 = 4;
pub const XDP_UMEM_FILL_RING: i32 = 5;
pub const XDP_UMEM_COMPLETION_RING: i32 = 6;
pub const XDP_STATISTICS: i32 = 7;
pub const XDP_OPTIONS: i32 = 8;

// mmap offsets for rings
pub const XDP_PGOFF_RX_RING: i64 = 0;
pub const XDP_PGOFF_TX_RING: i64 = 0x80000000;
pub const XDP_UMEM_PGOFF_FILL_RING: i64 = 0x100000000;
pub const XDP_UMEM_PGOFF_COMPLETION_RING: i64 = 0x180000000;

// XDP bind flags
pub const XDP_SHARED_UMEM: u16 = 1 << 0;
pub const XDP_COPY: u16 = 1 << 1;
pub const XDP_ZEROCOPY: u16 = 1 << 2;

// Default sizes
pub const DEFAULT_FRAME_SIZE: u32 = 4096;
pub const DEFAULT_FRAME_COUNT: u32 = 4096;
pub const DEFAULT_RING_SIZE: u32 = 2048;

/// sockaddr_xdp for bind()
#[repr(C)]
pub struct SockaddrXdp {
    pub sxdp_family: u16,
    pub sxdp_flags: u16,
    pub sxdp_ifindex: u32,
    pub sxdp_queue_id: u32,
    pub sxdp_shared_umem_fd: u32,
}

/// xdp_umem_reg for setsockopt(XDP_UMEM_REG)
#[repr(C)]
pub struct XdpUmemReg {
    pub addr: u64,
    pub len: u64,
    pub chunk_size: u32,
    pub headroom: u32,
    pub flags: u32,
}

/// xdp_desc for TX/RX ring entries
#[repr(C)]
#[derive(Clone, Copy)]
pub struct XdpDesc {
    pub addr: u64,
    pub len: u32,
    pub options: u32,
}

/// xdp_ring_offset (per-ring offsets returned from getsockopt)
#[repr(C)]
#[derive(Default)]
pub struct XdpRingOffset {
    pub producer: u64,
    pub consumer: u64,
    pub desc: u64,
    pub flags: u64,
}

/// xdp_mmap_offsets (returned from getsockopt XDP_MMAP_OFFSETS)
#[repr(C)]
#[derive(Default)]
pub struct XdpMmapOffsets {
    pub rx: XdpRingOffset,
    pub tx: XdpRingOffset,
    pub fr: XdpRingOffset, // fill ring
    pub cr: XdpRingOffset, // completion ring
}
