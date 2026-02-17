use std::sync::atomic::{AtomicU32, Ordering};

use super::consts::{XdpDesc, XdpRingOffset};

/// AF_XDP ring의 공통 구조.
/// producer/consumer 포인터와 descriptor 배열을 mmap된 메모리에서 참조한다.
pub struct Ring {
    pub(crate) producer: *mut AtomicU32,
    pub(crate) consumer: *mut AtomicU32,
    pub(crate) ring_base: *mut u8,
    pub(crate) size: u32,
    pub(crate) mask: u32,
}

impl Ring {
    /// mmap된 메모리 베이스와 ring offset 정보로 Ring을 초기화한다.
    pub unsafe fn from_mmap(base: *mut u8, offsets: &XdpRingOffset, size: u32) -> Self {
        Self {
            producer: base.add(offsets.producer as usize) as *mut AtomicU32,
            consumer: base.add(offsets.consumer as usize) as *mut AtomicU32,
            ring_base: base.add(offsets.desc as usize),
            size,
            mask: size - 1,
        }
    }

    fn producer_val(&self) -> u32 {
        unsafe { (*self.producer).load(Ordering::Acquire) }
    }

    fn consumer_val(&self) -> u32 {
        unsafe { (*self.consumer).load(Ordering::Acquire) }
    }
}

/// Fill Ring: 유저 → 커널로 빈 프레임 주소를 전달한다.
/// 커널이 수신한 패킷을 이 프레임에 채워넣는다.
pub struct FillRing {
    ring: Ring,
}

impl FillRing {
    pub unsafe fn new(base: *mut u8, offsets: &XdpRingOffset, size: u32) -> Self {
        Self {
            ring: Ring::from_mmap(base, offsets, size),
        }
    }

    /// 빈 프레임 주소를 Fill Ring에 추가한다.
    pub fn produce(&mut self, addr: u64) {
        let prod = self.ring.producer_val();
        let idx = prod & self.ring.mask;
        unsafe {
            let desc_ptr = self.ring.ring_base as *mut u64;
            *desc_ptr.add(idx as usize) = addr;
            (*self.ring.producer).store(prod.wrapping_add(1), Ordering::Release);
        }
    }

    /// 여러 프레임 주소를 한번에 추가한다.
    pub fn produce_batch(&mut self, addrs: &[u64]) {
        let mut prod = self.ring.producer_val();
        for &addr in addrs {
            let idx = prod & self.ring.mask;
            unsafe {
                let desc_ptr = self.ring.ring_base as *mut u64;
                *desc_ptr.add(idx as usize) = addr;
            }
            prod = prod.wrapping_add(1);
        }
        unsafe {
            (*self.ring.producer).store(prod, Ordering::Release);
        }
    }
}

/// Completion Ring: 커널 → 유저로 전송 완료된 프레임 주소를 반환한다.
pub struct CompletionRing {
    ring: Ring,
    cached_cons: u32,
}

impl CompletionRing {
    pub unsafe fn new(base: *mut u8, offsets: &XdpRingOffset, size: u32) -> Self {
        let ring = Ring::from_mmap(base, offsets, size);
        let cached_cons = ring.consumer_val();
        Self { ring, cached_cons }
    }

    /// 전송 완료된 프레임 주소를 꺼낸다.
    pub fn consume(&mut self) -> Option<u64> {
        let prod = self.ring.producer_val();
        if self.cached_cons == prod {
            return None;
        }
        let idx = self.cached_cons & self.ring.mask;
        let addr = unsafe {
            let desc_ptr = self.ring.ring_base as *const u64;
            *desc_ptr.add(idx as usize)
        };
        self.cached_cons = self.cached_cons.wrapping_add(1);
        unsafe {
            (*self.ring.consumer).store(self.cached_cons, Ordering::Release);
        }
        Some(addr)
    }
}

/// RX Ring: 커널 → 유저로 수신된 패킷 descriptor를 전달한다.
pub struct RxRing {
    ring: Ring,
    cached_cons: u32,
}

impl RxRing {
    pub unsafe fn new(base: *mut u8, offsets: &XdpRingOffset, size: u32) -> Self {
        let ring = Ring::from_mmap(base, offsets, size);
        let cached_cons = ring.consumer_val();
        Self { ring, cached_cons }
    }

    /// 수신된 패킷 descriptor를 꺼낸다.
    pub fn consume(&mut self) -> Option<XdpDesc> {
        let prod = self.ring.producer_val();
        if self.cached_cons == prod {
            return None;
        }
        let idx = self.cached_cons & self.ring.mask;
        let desc = unsafe {
            let desc_ptr = self.ring.ring_base as *const XdpDesc;
            *desc_ptr.add(idx as usize)
        };
        self.cached_cons = self.cached_cons.wrapping_add(1);
        unsafe {
            (*self.ring.consumer).store(self.cached_cons, Ordering::Release);
        }
        Some(desc)
    }
}

/// TX Ring: 유저 → 커널로 전송할 패킷 descriptor를 전달한다.
pub struct TxRing {
    ring: Ring,
}

impl TxRing {
    pub unsafe fn new(base: *mut u8, offsets: &XdpRingOffset, size: u32) -> Self {
        Self {
            ring: Ring::from_mmap(base, offsets, size),
        }
    }

    /// 전송할 패킷 descriptor를 TX Ring에 추가한다.
    pub fn produce(&mut self, desc: XdpDesc) {
        let prod = self.ring.producer_val();
        let idx = prod & self.ring.mask;
        unsafe {
            let desc_ptr = self.ring.ring_base as *mut XdpDesc;
            *desc_ptr.add(idx as usize) = desc;
            (*self.ring.producer).store(prod.wrapping_add(1), Ordering::Release);
        }
    }

    /// TX Ring에 여유가 있는지 확인한다.
    pub fn available(&self) -> u32 {
        let prod = self.ring.producer_val();
        let cons = self.ring.consumer_val();
        self.ring.size - (prod.wrapping_sub(cons))
    }
}

// SAFETY: Ring pointers are into mmap'd kernel memory, only accessed via
// atomic operations on producer/consumer indices.
unsafe impl Send for FillRing {}
unsafe impl Send for CompletionRing {}
unsafe impl Send for RxRing {}
unsafe impl Send for TxRing {}
