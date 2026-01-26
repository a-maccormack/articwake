use std::net::{TcpStream, ToSocketAddrs};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, serde::Serialize)]
pub struct HostStatus {
    pub reachable: bool,
    pub initrd_ssh_open: bool,
    pub system_ssh_open: bool,
}

pub fn check_host_status(ip: &str, initrd_ssh_port: u16) -> HostStatus {
    let reachable = ping_host(ip);
    let initrd_ssh_open = check_tcp_port(ip, initrd_ssh_port);
    let system_ssh_open = check_tcp_port(ip, 22);

    HostStatus {
        reachable,
        initrd_ssh_open,
        system_ssh_open,
    }
}

fn ping_host(ip: &str) -> bool {
    Command::new("ping")
        .args(["-c", "1", "-W", "2", ip])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn check_tcp_port(ip: &str, port: u16) -> bool {
    let addr = format!("{}:{}", ip, port);
    if let Ok(mut addrs) = addr.to_socket_addrs() {
        if let Some(addr) = addrs.next() {
            return TcpStream::connect_timeout(&addr, Duration::from_secs(3)).is_ok();
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_host_status_localhost() {
        // Localhost should be reachable
        let status = check_host_status("127.0.0.1", 2222);
        assert!(status.reachable);
    }

    #[test]
    fn test_check_host_status_unreachable() {
        // Non-routable IP should not be reachable (or timeout quickly)
        let status = check_host_status("192.0.2.1", 2222); // TEST-NET-1, should not route
        assert!(!status.initrd_ssh_open);
        assert!(!status.system_ssh_open);
    }

    #[test]
    fn test_check_tcp_port_closed() {
        // Port 59999 should not be open on localhost
        assert!(!check_tcp_port("127.0.0.1", 59999));
    }

    #[test]
    fn test_check_tcp_port_invalid_ip() {
        assert!(!check_tcp_port("not-an-ip", 80));
    }

    #[test]
    fn test_host_status_struct_serialization() {
        let status = HostStatus {
            reachable: true,
            initrd_ssh_open: false,
            system_ssh_open: true,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"reachable\":true"));
        assert!(json.contains("\"initrd_ssh_open\":false"));
        assert!(json.contains("\"system_ssh_open\":true"));
    }
}
