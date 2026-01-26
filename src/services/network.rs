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
