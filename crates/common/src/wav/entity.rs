#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavProperties {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub duration_ms: u64,
    pub data_offset: u64,
    pub data_bytes: u64,
}
