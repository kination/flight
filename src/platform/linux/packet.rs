use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

/// 완전한 Ethernet + IPv4 + UDP 프레임을 구성한다.
/// 반환: 실제 기록된 바이트 수
pub fn build_udp_frame(
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
    out: &mut [u8],
) -> Option<usize> {
    let total_len = 14 + 20 + 8 + payload.len(); // Eth + IPv4 + UDP + payload
    if out.len() < total_len {
        return None;
    }

    // --- Ethernet Header (14 bytes) ---
    out[0..6].copy_from_slice(&dst_mac);
    out[6..12].copy_from_slice(&src_mac);
    out[12] = 0x08; // EtherType: IPv4
    out[13] = 0x00;

    // --- IPv4 Header (20 bytes, no options) ---
    let ip_total = (20 + 8 + payload.len()) as u16;
    out[14] = 0x45; // Version=4, IHL=5
    out[15] = 0x00; // DSCP/ECN
    out[16..18].copy_from_slice(&ip_total.to_be_bytes());
    out[18..20].copy_from_slice(&0u16.to_be_bytes()); // Identification
    out[20..22].copy_from_slice(&0u16.to_be_bytes()); // Flags/Fragment
    out[22] = 64; // TTL
    out[23] = 17; // Protocol: UDP
    out[24..26].copy_from_slice(&0u16.to_be_bytes()); // Checksum (placeholder)
    out[26..30].copy_from_slice(&src_ip.octets());
    out[30..34].copy_from_slice(&dst_ip.octets());

    // IP header checksum
    let cksum = ip_checksum(&out[14..34]);
    out[24..26].copy_from_slice(&cksum.to_be_bytes());

    // --- UDP Header (8 bytes) ---
    let udp_len = (8 + payload.len()) as u16;
    out[34..36].copy_from_slice(&src_port.to_be_bytes());
    out[36..38].copy_from_slice(&dst_port.to_be_bytes());
    out[38..40].copy_from_slice(&udp_len.to_be_bytes());
    out[40..42].copy_from_slice(&0u16.to_be_bytes()); // UDP checksum (0 = disabled)

    // --- Payload ---
    out[42..42 + payload.len()].copy_from_slice(payload);

    Some(total_len)
}

/// 수신된 Ethernet 프레임에서 UDP payload와 송신자 정보를 파싱한다.
pub fn parse_udp_frame(frame: &[u8]) -> Option<(SocketAddr, &[u8])> {
    // 최소 42바이트 (Eth + IPv4 + UDP)
    if frame.len() < 42 {
        return None;
    }

    // EtherType == IPv4?
    if frame[12] != 0x08 || frame[13] != 0x00 {
        return None;
    }

    // Protocol == UDP?
    if frame[23] != 17 {
        return None;
    }

    let src_ip = Ipv4Addr::new(frame[26], frame[27], frame[28], frame[29]);
    let src_port = u16::from_be_bytes([frame[34], frame[35]]);
    let udp_len = u16::from_be_bytes([frame[38], frame[39]]) as usize;

    if udp_len < 8 || frame.len() < 34 + udp_len {
        return None;
    }

    let payload = &frame[42..34 + udp_len];
    let addr = SocketAddr::V4(SocketAddrV4::new(src_ip, src_port));

    Some((addr, payload))
}

fn ip_checksum(header: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i < header.len() - 1 {
        sum += u16::from_be_bytes([header[i], header[i + 1]]) as u32;
        i += 2;
    }
    if header.len() % 2 != 0 {
        sum += (header[header.len() - 1] as u32) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}
