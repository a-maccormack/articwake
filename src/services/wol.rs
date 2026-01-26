use std::net::UdpSocket;
use thiserror::Error;
use wake_on_lan::MagicPacket;

#[derive(Debug, Error)]
pub enum WolError {
    #[error("Invalid MAC address: {0}")]
    InvalidMac(String),
    #[error("Failed to send magic packet: {0}")]
    SendFailed(#[from] std::io::Error),
}

pub fn parse_mac(mac_str: &str) -> Result<[u8; 6], WolError> {
    let clean: String = mac_str.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() != 12 {
        return Err(WolError::InvalidMac(mac_str.to_string()));
    }

    let mut mac = [0u8; 6];
    for (i, chunk) in clean.as_bytes().chunks(2).enumerate() {
        let hex_str = std::str::from_utf8(chunk).unwrap();
        mac[i] = u8::from_str_radix(hex_str, 16)
            .map_err(|_| WolError::InvalidMac(mac_str.to_string()))?;
    }
    Ok(mac)
}

pub fn send_magic_packet(mac_str: &str, broadcast: &str) -> Result<(), WolError> {
    let mac = parse_mac(mac_str)?;
    let packet = MagicPacket::new(&mac);

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_broadcast(true)?;

    let dest = format!("{}:9", broadcast);
    socket.send_to(packet.magic_bytes(), &dest)?;

    tracing::info!("Sent WOL magic packet to {} via {}", mac_str, broadcast);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mac_colon_separated() {
        let mac = parse_mac("aa:bb:cc:dd:ee:ff").unwrap();
        assert_eq!(mac, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_parse_mac_dash_separated() {
        let mac = parse_mac("11-22-33-44-55-66").unwrap();
        assert_eq!(mac, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    }

    #[test]
    fn test_parse_mac_no_separator() {
        let mac = parse_mac("aabbccddeeff").unwrap();
        assert_eq!(mac, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_parse_mac_uppercase() {
        let mac = parse_mac("AA:BB:CC:DD:EE:FF").unwrap();
        assert_eq!(mac, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_parse_mac_mixed_case() {
        let mac = parse_mac("Aa:Bb:Cc:Dd:Ee:Ff").unwrap();
        assert_eq!(mac, [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
    }

    #[test]
    fn test_parse_mac_zeros() {
        let mac = parse_mac("00:00:00:00:00:00").unwrap();
        assert_eq!(mac, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_parse_mac_max_values() {
        let mac = parse_mac("ff:ff:ff:ff:ff:ff").unwrap();
        assert_eq!(mac, [0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn test_parse_mac_invalid_too_short() {
        assert!(parse_mac("aa:bb:cc:dd:ee").is_err());
        assert!(parse_mac("aabbccddee").is_err());
    }

    #[test]
    fn test_parse_mac_invalid_too_long() {
        assert!(parse_mac("aa:bb:cc:dd:ee:ff:00").is_err());
        assert!(parse_mac("aabbccddeeff00").is_err());
    }

    #[test]
    fn test_parse_mac_invalid_chars() {
        assert!(parse_mac("gg:hh:ii:jj:kk:ll").is_err());
        assert!(parse_mac("not-valid").is_err());
    }

    #[test]
    fn test_parse_mac_empty() {
        assert!(parse_mac("").is_err());
    }

    #[test]
    fn test_send_magic_packet_valid() {
        // This test actually sends a packet to localhost broadcast
        // It should succeed without errors
        let result = send_magic_packet("aa:bb:cc:dd:ee:ff", "127.255.255.255");
        assert!(result.is_ok());
    }

    #[test]
    fn test_send_magic_packet_invalid_mac() {
        let result = send_magic_packet("invalid", "127.255.255.255");
        assert!(result.is_err());
    }
}
