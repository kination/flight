use crate::error::FlightError;

/// UMEM: AF_XDP에서 커널과 유저스페이스가 공유하는 메모리 영역.
/// mmap으로 할당하고, AF_XDP 소켓에 등록한다.
pub struct Umem {
    pub(crate) ptr: *mut u8,
    pub(crate) size: usize,
    pub(crate) frame_size: usize,
    pub(crate) frame_count: usize,
}

impl Umem {
    pub fn new(frame_count: usize, frame_size: usize) -> Result<Self, FlightError> {
        let size = frame_count * frame_size;

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(FlightError::Bind("UMEM mmap failed".into()));
        }

        Ok(Self {
            ptr: ptr as *mut u8,
            size,
            frame_size,
            frame_count,
        })
    }

    /// UMEM 내 특정 프레임 주소의 포인터를 반환한다.
    pub fn frame_ptr(&self, addr: u64) -> *mut u8 {
        unsafe { self.ptr.add(addr as usize) }
    }
}

impl Drop for Umem {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                libc::munmap(self.ptr as *mut libc::c_void, self.size);
            }
        }
    }
}

// SAFETY: Umem is a raw memory region accessed with proper synchronization
// via the AF_XDP ring buffers. Send is needed for ownership transfer.
unsafe impl Send for Umem {}
