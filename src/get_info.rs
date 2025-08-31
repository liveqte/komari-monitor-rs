use crate::data_struct::{Connections, Cpu, Disk, Load, Network, Ram, Swap};
use std::collections::HashSet;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use sysinfo::{Disks, Networks, System};
use tokio::task::JoinHandle;
use ureq::config::IpFamily;

pub fn arch() -> String {
    std::env::consts::ARCH.to_string()
}

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
    let name = hashset.into_iter().collect::<Vec<String>>().join(", ");

    CPUInfoWithOutUsage { name, cores }
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

    MemDiskInfoWithOutUsage {
        mem_total,
        swap_total,
        disk_total: all_disk_space,
    }
}

pub struct IPInfo {
    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>,
}

pub async fn ip() -> IPInfo {
    let ipv4: JoinHandle<Option<Ipv4Addr>> = tokio::spawn(async move {
        let Ok(mut resp) = ureq::get("https://www.cloudflare.com/cdn-cgi/trace")
            .header("User-Agent", "curl/8.7.1")
            .config()
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

    IPInfo {
        ipv4: ipv4.await.unwrap(),
        ipv6: ipv6.await.unwrap(),
    }
}

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

    OsInfo {
        os,
        version: kernel_version,
        virtualization: virt,
    }
}

pub fn realtime_cpu(sysinfo_sys: &System) -> Cpu {
    let cpus = sysinfo_sys.cpus();
    let mut avg = 0.0;
    for cpu in cpus {
        avg += cpu.cpu_usage();
    }
    let avg = f64::from(avg) / cpus.len() as f64;

    Cpu { usage: avg }
}

pub fn realtime_mem(sysinfo_sys: &System) -> Ram {
    Ram {
        used: sysinfo_sys.total_memory() - sysinfo_sys.available_memory(),
    }
}

pub fn realtime_swap(sysinfo_sys: &System) -> Swap {
    Swap {
        used: sysinfo_sys.used_swap(),
    }
}

pub fn realtime_disk(disk: &Disks) -> Disk {
    let mut used_disk: u64 = 0;
    let disk_list = filter_disks(disk);
    for disk in disk_list {
        used_disk += disk.total_space() - disk.available_space();
    }

    Disk { used: used_disk }
}

#[cfg(not(target_os = "windows"))]
pub fn realtime_load() -> Load {
    let load = System::load_average();
    Load {
        load1: load.one,
        load5: load.five,
        load15: load.fifteen,
    }
}

#[cfg(target_os = "windows")]
pub fn realtime_load() -> Load {
    Load {
        load1: 0.0,
        load5: 0.0,
        load15: 0.0,
    }
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
        Network {
            up: (up as f64 / (DURATION / 1000.0)) as u64,
            down: (down as f64 / (DURATION / 1000.0)) as u64,
            total_up,
            total_down,
        }
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
    Connections {
        tcp: tcp4 + tcp6,
        udp: udp4 + udp6,
    }
}

#[cfg(target_os = "windows")]
pub fn realtime_connections() -> Connections {
    use netstat2::{iterate_sockets_info_without_pids, ProtocolFlags, ProtocolSocketInfo};
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    let Ok(sockets_iterator) = iterate_sockets_info_without_pids(proto_flags) else {
        return Connections { tcp: 0, udp: 0 };
    };

    let (mut tcp_count, mut udp_count) = (0, 0);

    for info_result in sockets_iterator.flatten() {
        match info_result.protocol_socket_info {
            ProtocolSocketInfo::Tcp(_) => tcp_count += 1,
            ProtocolSocketInfo::Udp(_) => udp_count += 1,
        }
    }

    Connections {
        tcp: tcp_count,
        udp: udp_count,
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn realtime_connections() -> Connections {
    Connections { tcp: 0, udp: 0 }
}

pub fn realtime_uptime() -> u64 {
    System::uptime()
}

pub fn realtime_process() -> u64 {
    let mut process_count = 0;

    let Ok(entries) = fs::read_dir("/proc") else {
        return 0;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        if let Some(name_str) = file_name.to_str() {
            if name_str.parse::<u32>().is_ok() {
                process_count += 1;
            }
        }
    }

    process_count as u64
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
