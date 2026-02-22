use std::collections::VecDeque;

/// UMEM 프레임 인덱스를 관리하는 free-list 할당자.
pub struct FrameAllocator {
    free: VecDeque<u64>,
}

impl FrameAllocator {
    pub fn new(frame_count: usize, frame_size: usize) -> Self {
        let free = (0..frame_count).map(|i| (i * frame_size) as u64).collect();
        Self { free }
    }

    /// 사용 가능한 프레임 주소를 할당한다.
    pub fn alloc(&mut self) -> Option<u64> {
        self.free.pop_front()
    }

    /// 사용이 끝난 프레임 주소를 반환한다.
    pub fn free(&mut self, addr: u64) {
        self.free.push_back(addr);
    }

    pub fn available(&self) -> usize {
        self.free.len()
    }
}
