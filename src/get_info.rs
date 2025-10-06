use crate::command_parser::IpProvider;
use crate::data_struct::{Connections, Cpu, Disk, Load, Network, Ram, Swap};
use log::trace;
use miniserde::{Deserialize, Serialize, json};
use std::collections::HashSet;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::time::Duration;
use sysinfo::{Disks, Networks, System};
use tokio::task::JoinHandle;
use ureq::config::IpFamily;

pub fn arch() -> String {
    let arch = std::env::consts::ARCH.to_string();
    trace!("ARCH 获取成功: {arch}");
    arch
}

#[derive(Debug)]
pub struct CPUInfoWithOutUsage {
    pub name: String,
    pub cores: u16,
}

pub fn cpu_info_without_usage(sysinfo_sys: &System) -> CPUInfoWithOutUsage {
    let cores = u16::try_from(sysinfo_sys.cpus().len()).unwrap_or(0);
    let mut hashset = HashSet::new();
    for cpu in sysinfo_sys.cpus() {
        hashset.insert(cpu.brand().to_string());
    }
    let name = hashset
        .into_iter()
        .collect::<Vec<String>>()
        .join(", ")
        .trim()
        .to_string();

    let cpu_info = CPUInfoWithOutUsage { name, cores };

    trace!("CPU INFO WITH OUT USAGE 获取成功: {cpu_info:?}");

    cpu_info
}

#[derive(Debug)]
pub struct MemDiskInfoWithOutUsage {
    pub mem_total: u64,
    pub swap_total: u64,
    pub disk_total: u64,
}

pub fn mem_info_without_usage(sysinfo_sys: &System) -> MemDiskInfoWithOutUsage {
    let mem_total = sysinfo_sys.total_memory();
    let swap_total = sysinfo_sys.total_swap();

    let disks = Disks::new_with_refreshed_list();
    let disk_list = filter_disks(&disks);
    let mut all_disk_space: u64 = 0;
    for disk in &disk_list {
        all_disk_space += disk.total_space();
    }

    let info = MemDiskInfoWithOutUsage {
        mem_total,
        swap_total,
        disk_total: all_disk_space,
    };

    trace!("MEM DISK INFO WITH OUT USAGE 获取成功: {info:?}");

    info
}

pub async fn ip(provider: &IpProvider) -> IPInfo {
    match provider {
        IpProvider::Cloudflare => ip_cloudflare().await,
        IpProvider::Ipinfo => ip_ipinfo().await,
    }
}

#[derive(Debug)]
pub struct IPInfo {
    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>,
}

pub async fn ip_ipinfo() -> IPInfo {
    let ipv4: JoinHandle<Option<Ipv4Addr>> = tokio::spawn(async move {
        let Ok(mut resp) = ureq::get("https://ipinfo.io")
            .header("User-Agent", "curl/8.7.1")
            .config()
            .timeout_global(Some(Duration::from_secs(5)))
            .ip_family(IpFamily::Ipv4Only)
            .build()
            .call()
        else {
            return None;
        };

        let Ok(body) = resp.body_mut().read_to_string() else {
            return None;
        };

        #[derive(Serialize, Deserialize)]
        struct IpJson {
            ip: String,
        }

        let json: IpJson = match json::from_str(&body) {
            Err(_) => {
                return None;
            }
            Ok(json) => json,
        };

        Ipv4Addr::from_str(json.ip.as_str()).ok()
    });

    let ipv6: JoinHandle<Option<Ipv6Addr>> = tokio::spawn(async move {
        let Ok(mut resp) = ureq::get("https://6.ipinfo.io")
            .header("User-Agent", "curl/8.7.1")
            .config()
            .timeout_global(Some(Duration::from_secs(5)))
            .ip_family(IpFamily::Ipv6Only)
            .build()
            .call()
        else {
            return None;
        };

        let Ok(body) = resp.body_mut().read_to_string() else {
            return None;
        };

        #[derive(Serialize, Deserialize)]
        struct IpJson {
            ip: String,
        }

        let json: IpJson = match json::from_str(&body) {
            Err(_) => {
                return None;
            }
            Ok(json) => json,
        };

        Ipv6Addr::from_str(json.ip.as_str()).ok()
    });

    let ip_info = IPInfo {
        ipv4: ipv4.await.unwrap(),
        ipv6: ipv6.await.unwrap(),
    };

    trace!("IP INFO (ipinfo) 获取成功: {ip_info:?}");

    ip_info
}

pub async fn ip_cloudflare() -> IPInfo {
    let ipv4: JoinHandle<Option<Ipv4Addr>> = tokio::spawn(async move {
        let Ok(mut resp) = ureq::get("https://www.cloudflare.com/cdn-cgi/trace")
            .header("User-Agent", "curl/8.7.1")
            .config()
            .timeout_global(Some(Duration::from_secs(5)))
            .ip_family(IpFamily::Ipv4Only)
            .build()
            .call()
        else {
            return None;
        };

        let Ok(body) = resp.body_mut().read_to_string() else {
            return None;
        };

        let mut ip = String::new();

        for line in body.lines() {
            if line.starts_with("ip=") {
                ip = line.replace("ip=", "");
                break;
            }
        }

        Ipv4Addr::from_str(ip.as_str()).ok()
    });

    let ipv6: JoinHandle<Option<Ipv6Addr>> = tokio::spawn(async move {
        let Ok(mut resp) = ureq::get("https://www.cloudflare.com/cdn-cgi/trace")
            .header("User-Agent", "curl/8.7.1")
            .config()
            .timeout_global(Some(Duration::from_secs(5)))
            .ip_family(IpFamily::Ipv6Only)
            .build()
            .call()
        else {
            return None;
        };

        let Ok(body) = resp.body_mut().read_to_string() else {
            return None;
        };

        let mut ip = String::new();

        for line in body.lines() {
            if line.starts_with("ip=") {
                ip = line.replace("ip=", "");
                break;
            }
        }

        Ipv6Addr::from_str(ip.as_str()).ok()
    });

    let ip_info = IPInfo {
        ipv4: ipv4.await.unwrap(),
        ipv6: ipv6.await.unwrap(),
    };

    trace!("IP INFO (cloudflare) 获取成功: {ip_info:?}");

    ip_info
}

#[derive(Debug)]
pub struct OsInfo {
    pub os: String,
    pub version: String,
    pub virtualization: String,
}

pub async fn os() -> OsInfo {
    let os = format!(
        "{} {}",
        System::name().unwrap_or_default(),
        System::os_version().unwrap_or_default()
    );
    let kernel_version = System::kernel_version().unwrap_or("Unknown".to_string());

    let virt = {
        #[cfg(target_os = "linux")]
        {
            heim_virt::detect()
                .await
                .unwrap_or(heim_virt::Virtualization::Unknown)
                .as_str()
                .to_string()
        }

        #[cfg(target_os = "windows")]
        {
            use raw_cpuid::CpuId;
            let hypervisor_present = {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                {
                    CpuId::new()
                        .get_feature_info()
                        .is_some_and(|f| f.has_hypervisor())
                }
                #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
                {
                    false
                }
            };

            let hypervisor_vendor = {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                {
                    if hypervisor_present {
                        CpuId::new()
                            .get_hypervisor_info()
                            .map(|hv| format!("{:?}", hv.identify()))
                    } else {
                        None
                    }
                }
                #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
                {
                    None
                }
            };

            hypervisor_vendor.unwrap_or_else(|| "Unknown".to_string())
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            "Unknown".to_string()
        }
    };

    let os_info = OsInfo {
        os,
        version: kernel_version,
        virtualization: virt,
    };

    trace!("OS INFO 获取成功: {os_info:?}");

    os_info
}

pub fn realtime_cpu(sysinfo_sys: &System) -> Cpu {
    let cpus = sysinfo_sys.cpus();
    let mut avg = 0.0;
    for cpu in cpus {
        avg += cpu.cpu_usage();
    }
    let avg = f64::from(avg) / cpus.len() as f64;

    let cpu = Cpu { usage: avg };
    trace!("REALTIME CPU 获取成功: {cpu:?}");
    cpu
}

pub fn realtime_mem(sysinfo_sys: &System) -> Ram {
    let ram = Ram {
        used: sysinfo_sys.total_memory() - sysinfo_sys.available_memory(),
    };
    trace!("REALTIME MEM 获取成功: {ram:?}");
    ram
}

pub fn realtime_swap(sysinfo_sys: &System) -> Swap {
    let swap = Swap {
        used: sysinfo_sys.used_swap(),
    };
    trace!("REALTIME SWAP 获取成功: {swap:?}");
    swap
}

pub fn realtime_disk(disk: &Disks) -> Disk {
    let mut used_disk: u64 = 0;
    let disk_list = filter_disks(disk);
    for disk in disk_list {
        used_disk += disk.total_space() - disk.available_space();
    }

    let disk_info = Disk { used: used_disk };
    trace!("REALTIME DISK 获取成功: {disk_info:?}");
    disk_info
}

#[cfg(not(target_os = "windows"))]
pub fn realtime_load() -> Load {
    let load = System::load_average();
    let load_info = Load {
        load1: load.one,
        load5: load.five,
        load15: load.fifteen,
    };
    trace!("REALTIME LOAD 获取成功: {:?}", load_info);
    load_info
}

#[cfg(target_os = "windows")]
pub fn realtime_load() -> Load {
    let load_info = Load {
        load1: 0.0,
        load5: 0.0,
        load15: 0.0,
    };
    trace!("REALTIME LOAD 获取成功: {load_info:?}");
    load_info
}

pub static mut DURATION: f64 = 0.0;

pub fn realtime_network(network: &Networks) -> Network {
    let mut total_up = 0;
    let mut total_down = 0;
    let mut up = 0;
    let mut down = 0;

    for (name, data) in network {
        if name.contains("lo")
            || name.contains("veth")
            || name.contains("docker")
            || name.contains("tun")
            || name.contains("br")
            || name.contains("tap")
        {
            continue;
        }
        total_up += data.total_transmitted();
        total_down += data.total_received();
        up += data.transmitted();
        down += data.received();
    }

    unsafe {
        let network_info = Network {
            up: (up as f64 / (DURATION / 1000.0)) as u64,
            down: (down as f64 / (DURATION / 1000.0)) as u64,
            total_up,
            total_down,
        };
        trace!("REALTIME NETWORK 获取成功: {network_info:?}");
        network_info
    }
}

#[cfg(target_os = "linux")]
pub fn realtime_connections() -> Connections {
    use crate::netlink::connections_count_with_protocol;
    let tcp4 =
        connections_count_with_protocol(libc::AF_INET as u8, libc::IPPROTO_TCP as u8).unwrap_or(0);
    let tcp6 =
        connections_count_with_protocol(libc::AF_INET6 as u8, libc::IPPROTO_TCP as u8).unwrap_or(0);
    let udp4 =
        connections_count_with_protocol(libc::AF_INET as u8, libc::IPPROTO_UDP as u8).unwrap_or(0);
    let udp6 =
        connections_count_with_protocol(libc::AF_INET6 as u8, libc::IPPROTO_UDP as u8).unwrap_or(0);
    let connections = Connections {
        tcp: tcp4 + tcp6,
        udp: udp4 + udp6,
    };
    trace!("REALTIME CONNECTIONS 获取成功: {:?}", connections);
    connections
}

#[cfg(target_os = "windows")]
pub fn realtime_connections() -> Connections {
    use netstat2::{ProtocolFlags, ProtocolSocketInfo, iterate_sockets_info_without_pids};
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    let Ok(sockets_iterator) = iterate_sockets_info_without_pids(proto_flags) else {
        let connections = Connections { tcp: 0, udp: 0 };
        trace!("REALTIME CONNECTIONS 获取成功: {connections:?}");
        return connections;
    };

    let (mut tcp_count, mut udp_count) = (0, 0);

    for info_result in sockets_iterator.flatten() {
        match info_result.protocol_socket_info {
            ProtocolSocketInfo::Tcp(_) => tcp_count += 1,
            ProtocolSocketInfo::Udp(_) => udp_count += 1,
        }
    }

    let connections = Connections {
        tcp: tcp_count,
        udp: udp_count,
    };
    trace!("REALTIME CONNECTIONS 获取成功: {connections:?}");
    connections
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn realtime_connections() -> Connections {
    let connections = Connections { tcp: 0, udp: 0 };
    trace!("REALTIME CONNECTIONS 获取成功: {:?}", connections);
    connections
}

pub fn realtime_uptime() -> u64 {
    let uptime = System::uptime();
    trace!("REALTIME UPTIME 获取成功: {uptime}");
    uptime
}

pub fn realtime_process() -> u64 {
    let mut process_count = 0;

    let Ok(entries) = fs::read_dir("/proc") else {
        trace!("REALTIME PROCESS 获取失败: 无法读取 /proc 目录");
        return 0;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        if let Some(name_str) = file_name.to_str()
            && name_str.parse::<u32>().is_ok()
        {
            process_count += 1;
        }
    }

    let process_count = process_count as u64;
    trace!("REALTIME PROCESS 获取成功: {process_count}");
    process_count
}

pub fn filter_disks(disks: &Disks) -> Vec<&sysinfo::Disk> {
    let allowed = [
        "apfs",
        "ext4",
        "ext3",
        "ext2",
        "f2fs",
        "reiserfs",
        "jfs",
        "btrfs",
        "fuseblk",
        "zfs",
        "simfs",
        "ntfs",
        "fat32",
        "exfat",
        "xfs",
        "fuse.rclone",
    ];

    let filtered: Vec<&sysinfo::Disk> = disks
        .iter() // 返回 &Disk
        .filter(|disk| {
            let fs = disk.file_system().to_string_lossy();
            allowed.contains(&fs.as_ref())
        })
        .collect();

    filtered
}
