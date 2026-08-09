#[derive(Debug, Clone)]
pub struct XpressoConfig {
    pub ring_size: usize,
    pub frame_size: usize,
    pub zero_copy: bool,
}
impl Default for XpressoConfig {
    fn default() -> Self {
        Self {
            ring_size: 2048,
            frame_size: 4096,
            zero_copy: false,
        }
    }
}
