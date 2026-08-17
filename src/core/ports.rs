use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct PortBinding {
    pub port: u16,
    pub protocol: String, // "TCP" / "UDP" / "TCP6" / "UDP6"
    pub ip: String,       // "127.0.0.1", "0.0.0.0", "::1", "::"
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub cmd: Option<String>,
    pub cwd: Option<String>,
    pub user: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct SocketInfo {
    port: u16,
    proto: String,
    ip: String,
    inode: u64,
}

pub fn scan_listening_ports() -> Vec<PortBinding> {
    #[cfg(target_os = "linux")]
    {
        scan_linux_ports()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "linux")]
fn scan_linux_ports() -> Vec<PortBinding> {
    let mut sockets_by_inode: HashMap<u64, SocketInfo> = HashMap::new();

    // 1. Parse TCP and TCP6 sockets (State 0A = LISTEN)
    parse_proc_net_tcp("/proc/net/tcp", "TCP", &mut sockets_by_inode);
    parse_proc_net_tcp("/proc/net/tcp6", "TCP6", &mut sockets_by_inode);

    // 2. Map Inode -> PID by scanning /proc/[pid]/fd/*
    let mut inode_to_pid: HashMap<u64, u32> = HashMap::new();
    if let Ok(proc_dir) = fs::read_dir("/proc") {
        for entry in proc_dir.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if let Ok(pid) = name_str.parse::<u32>() {
                let fd_path = format!("/proc/{}/fd", pid);
                if let Ok(fd_entries) = fs::read_dir(&fd_path) {
                    for fd_entry in fd_entries.flatten() {
                        if let Ok(target) = fs::read_link(fd_entry.path()) {
                            let target_str = target.to_string_lossy();
                            if target_str.starts_with("socket:[") && target_str.ends_with(']') {
                                let inode_str = &target_str[8..target_str.len() - 1];
                                if let Ok(inode) = inode_str.parse::<u64>() {
                                    if sockets_by_inode.contains_key(&inode) {
                                        inode_to_pid.insert(inode, pid);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Assemble PortBinding results
    let mut bindings: Vec<PortBinding> = Vec::new();
    for (inode, socket) in sockets_by_inode {
        let pid = inode_to_pid.get(&inode).copied();
        bindings.push(PortBinding {
            port: socket.port,
            protocol: socket.proto,
            ip: socket.ip,
            pid,
            process_name: None,
            cmd: None,
            cwd: None,
            user: None,
        });
    }

    // Sort by port ascending
    bindings.sort_by(|a, b| {
        a.port
            .cmp(&b.port)
            .then_with(|| a.protocol.cmp(&b.protocol))
            .then_with(|| a.ip.cmp(&b.ip))
    });

    bindings
}

#[cfg(target_os = "linux")]
fn parse_proc_net_tcp(path: &str, proto: &str, out: &mut HashMap<u64, SocketInfo>) {
    let file = match File::open(Path::new(path)) {
        Ok(f) => f,
        Err(_) => return,
    };
    let reader = BufReader::new(file);

    for (i, line) in reader.lines().flatten().enumerate() {
        if i == 0 {
            continue; // Skip header line
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }

        // parts[1] is local_address (HEX_IP:HEX_PORT)
        // parts[3] is state ("0A" = LISTEN for TCP)
        // parts[9] is inode
        let state = parts[3];
        if state != "0A" {
            continue; // Only listen sockets
        }

        let local_addr = parts[1];
        let inode_str = parts[9];

        let inode: u64 = match inode_str.parse() {
            Ok(val) => val,
            Err(_) => continue,
        };

        if let Some((ip, port)) = parse_hex_addr(local_addr) {
            out.insert(
                inode,
                SocketInfo {
                    port,
                    proto: proto.to_string(),
                    ip,
                    inode,
                },
            );
        }
    }
}

#[cfg(target_os = "linux")]
fn parse_hex_addr(addr: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let hex_ip = parts[0];
    let hex_port = parts[1];

    let port = u16::from_str_radix(hex_port, 16).ok()?;

    if hex_ip.len() == 8 {
        // IPv4 (e.g. 0100007F -> 127.0.0.1)
        let num = u32::from_str_radix(hex_ip, 16).ok()?;
        let b1 = (num & 0xFF) as u8;
        let b2 = ((num >> 8) & 0xFF) as u8;
        let b3 = ((num >> 16) & 0xFF) as u8;
        let b4 = ((num >> 24) & 0xFF) as u8;
        Some((format!("{}.{}.{}.{}", b1, b2, b3, b4), port))
    } else if hex_ip.len() == 32 {
        // IPv6
        if hex_ip == "00000000000000000000000000000000" {
            Some(("::".to_string(), port))
        } else if hex_ip == "00000000000000000000000001000000" {
            Some(("::1".to_string(), port))
        } else {
            Some(("IPv6".to_string(), port))
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_parse_hex_addr_ipv4() {
        // 0100007F:0050 -> 127.0.0.1:80
        let (ip, port) = parse_hex_addr("0100007F:0050").expect("Should parse valid IPv4 hex");
        assert_eq!(ip, "127.0.0.1");
        assert_eq!(port, 80);

        // 00000000:0BB8 -> 0.0.0.0:3000
        let (ip, port) = parse_hex_addr("00000000:0BB8").expect("Should parse valid IPv4 hex");
        assert_eq!(ip, "0.0.0.0");
        assert_eq!(port, 3000);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_parse_hex_addr_ipv6() {
        // ::1:8080
        let (ip, port) = parse_hex_addr("00000000000000000000000001000000:1F90").expect("Should parse IPv6 localhost");
        assert_eq!(ip, "::1");
        assert_eq!(port, 8080);

        // :::22
        let (ip, port) = parse_hex_addr("00000000000000000000000000000000:0016").expect("Should parse IPv6 all");
        assert_eq!(ip, "::");
        assert_eq!(port, 22);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_parse_hex_addr_invalid() {
        assert!(parse_hex_addr("invalid").is_none());
        assert!(parse_hex_addr("127.0.0.1:80").is_none());
        assert!(parse_hex_addr("ZZZZZZZZ:0050").is_none());
    }
}
