/// Xpresso Protocol Header definitions
///
/// See docs/DESIGN.md §2 for the authoritative wire format.
///
/// XpressoHeader (20 bytes):
///   - Magic (u16, 0x5850 "XP")
///   - Version (4b) & Type (4b)
///   - Flags (u8)
///   - Connection ID (u64)
///   - Sequence Number (u32)
///   - Payload Length (u32)

pub const XPRESSO_MAGIC: u16 = 0x5850;
pub const XPRESSO_VERSION: u8 = 0x1;
pub const XPRESSO_HEADER_LEN: usize = 20;

/// v0.x defines only DATA. CONTROL is reserved for future use (receivers
/// MUST drop in v0.1). CONNECT/ACK/CLOSE are explicitly out of scope —
/// Xpresso is a datagram socket, not a connection-oriented protocol.
pub enum PacketType {
    Data,
    Control,
}

// TODO: 20바이트 크기의 XpressoHeader 구조체를 정의하세요.
// 필드 구성:
// - magic (u16): 0x5850 ("XP")
// - version_and_type (u8): 상위 4비트는 Version, 하위 4비트는 PacketType
// - flags (u8): 추가 옵션 플래그 (bit 0 = HAS_PADDING, 나머지 reserved)
// - connection_id (u64): 세션 핸들. 0은 미할당. OsRng로 생성.
// - seq_num (u32): 시퀀스 번호
// - payload_len (u32): 페이로드 길이
pub struct XpressoHeader {
    pub magic: u16,
    pub version_and_type: u8,
    pub flags: u8,
    pub connection_id: u64,
    pub seq_num: u32,
    pub payload_len: u32,
}

impl XpressoHeader {
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < XPRESSO_HEADER_LEN {
            return Err("buffer too short for XpressoHeader");
        }

        let magic = u16::from_be_bytes([data[0], data[1]]);
        if magic != XPRESSO_MAGIC {
            return Err("invalid magic number");
        }

        Ok(Self {
            magic,
            version_and_type: data[2],
            flags: data[3],
            connection_id: u64::from_be_bytes(data[4..12].try_into().unwrap()),
            seq_num: u32::from_be_bytes(data[12..16].try_into().unwrap()),
            payload_len: u32::from_be_bytes(data[16..20].try_into().unwrap()),
        })
    }

    pub fn to_bytes(&self) -> [u8; XPRESSO_HEADER_LEN] {
        let mut bytes = [0u8; XPRESSO_HEADER_LEN];
        bytes[0..2].copy_from_slice(&self.magic.to_be_bytes());
        bytes[2] = self.version_and_type;
        bytes[3] = self.flags;
        bytes[4..12].copy_from_slice(&self.connection_id.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.seq_num.to_be_bytes());
        bytes[16..20].copy_from_slice(&self.payload_len.to_be_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xpresso_header_serde() {
        // TODO: 정의한 XpressoHeader 구조체가 올바르게 직렬화 및 역직렬화 되는지
        // 확인하는 단위 테스트를 작성하세요.
    }
}
