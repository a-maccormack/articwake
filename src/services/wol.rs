use thiserror::Error;
use wake_on_lan::MagicPacket;
use std::net::UdpSocket;

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
