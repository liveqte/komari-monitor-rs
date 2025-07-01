use crate::data_struct::{CPU, Connections, Disk, Load, Network, RAM, Swap};
use reqwest::Client;
use std::collections::HashSet;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::time::Duration;
use sysinfo::{Disks, System};
use systemstat::{NetworkStats, Platform};

pub fn arch() -> String {
    let arch = std::env::consts::ARCH;
    arch.to_string()
}

pub struct CPUInfoWithOutUsage {
    pub name: String,
    pub cores: u16,
}
pub fn cpu_info_without_usage(sysinfo_sys: &System) -> CPUInfoWithOutUsage {
    let cores = sysinfo_sys.cpus().len() as u16;
    let mut hashset = HashSet::new();
    for cpu in sysinfo_sys.cpus() {
        hashset.insert(cpu.brand().to_string());
    }
    let name = hashset.into_iter().collect::<Vec<String>>().join(", ");

    CPUInfoWithOutUsage { name, cores }
}

pub struct MemDiskInfoWithOutUsage {
    pub mem_total: u64,
    pub swap_total: u64,
    pub disk_total: u64,
}

pub fn mem_info_without_usage(sysinfo_sys: &System) -> MemDiskInfoWithOutUsage {
    let mem_total = sysinfo_sys.total_memory();
    let swap_total = sysinfo_sys.total_swap();

    let disk_list = Disks::new_with_refreshed_list();
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

pub async fn ip(client: &Client) -> IPInfo {
    let ipv4_client = client.clone();
    let ipv6_client = client.clone();
    let ipv4 = tokio::spawn(async move {
        let resp = if let Ok(resp) = ipv4_client.get("https://4.ipinfo.io/").send().await {
            resp
        } else {
            return None;
        };

        let json = if let Ok(json) = resp.json::<serde_json::Value>().await {
            json
        } else {
            return None;
        };

        let ip = if let Some(ip) = json.get("ip") {
            if let Ok(ipv4) = Ipv4Addr::from_str(ip.as_str().unwrap_or("0.0.0.0")) {
                Some(ipv4)
            } else {
                None
            }
        } else {
            None
        };
        ip
    });

    let ipv6 = tokio::spawn(async move {
        let resp = if let Ok(resp) = ipv6_client.get("https://6.ipinfo.io/").send().await {
            resp
        } else {
            return None;
        };

        let json = if let Ok(json) = resp.json::<serde_json::Value>().await {
            json
        } else {
            return None;
        };

        let ip = if let Some(ip) = json.get("ip") {
            if let Ok(ipv6) = Ipv6Addr::from_str(ip.as_str().unwrap_or("0.0.0.0")) {
                Some(ipv6)
            } else {
                None
            }
        } else {
            None
        };
        ip
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
    let os = whoami::distro().unwrap_or("Unknown".to_string());
    let kernel_version = System::kernel_version().unwrap_or("Unknown".to_string());

    #[cfg(target_os = "linux")]
    let virt = heim_virt::detect()
        .await
        .unwrap_or(heim_virt::Virtualization::Unknown)
        .as_str()
        .to_string();

    #[cfg(not(target_os = "linux"))]
    let virt = "Unknown".to_string();

    OsInfo {
        os,
        version: kernel_version,
        virtualization: virt,
    }
}

pub fn realtime_cpu(sysinfo_sys: &System) -> CPU {
    let cpus = sysinfo_sys.cpus();
    let mut avg = 0.0;
    for cpu in cpus {
        avg += cpu.cpu_usage();
    }
    let avg = avg as f64 / cpus.len() as f64;

    CPU { usage: avg }
}

pub fn realtime_mem(sysinfo_sys: &System) -> RAM {
    RAM {
        total: sysinfo_sys.total_memory(),
        used: sysinfo_sys.total_memory() - sysinfo_sys.available_memory(),
    }
}

pub fn realtime_swap(sysinfo_sys: &System) -> Swap {
    Swap {
        total: sysinfo_sys.total_swap(),
        used: sysinfo_sys.used_swap(),
    }
}

pub fn realtime_disk() -> Disk {
    let disk_list = Disks::new_with_refreshed_list();
    let mut total_disk: u64 = 0;
    let mut used_disk: u64 = 0;
    for disk in &disk_list {
        total_disk += disk.total_space();
        used_disk += disk.total_space() - disk.available_space();
    }

    Disk {
        total: total_disk,
        used: used_disk,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn realtime_load(systemstat_sys: &systemstat::System) -> Load {
    let load_average = if let Ok(load) = systemstat_sys.load_average() {
        load
    } else {
        return Load {
            load1: 0.0,
            load5: 0.0,
            load15: 0.0,
        };
    };
    Load {
        load1: load_average.one as f64,
        load5: load_average.five as f64,
        load15: load_average.fifteen as f64,
    }
}

#[cfg(target_os = "windows")]
pub async fn realtime_load(_systemstat_sys: systemstat::System) -> Load {
    Load {
        load1: 0.0,
        load5: 0.0,
        load15: 0.0,
    }
}

static mut TMP_RX: u64 = 0;
static mut TMP_TX: u64 = 0;

pub fn realtime_network(systemstat_sys: &systemstat::System) -> Network {
    let nic = if let Ok(nic) = systemstat_sys.networks() {
        nic
    } else {
        return Network {
            up: 0,
            down: 0,
            total_up: 0,
            total_down: 0,
        };
    };

    let mut nic_names: Vec<String> = Vec::new();
    for (name, _) in nic {
        nic_names.push(name);
    }
    let mut all_rx: u64 = 0;
    let mut all_tx: u64 = 0;
    for nic_name in nic_names {
        let network_stats =
            systemstat_sys
                .network_stats(nic_name.as_str())
                .unwrap_or(NetworkStats {
                    rx_bytes: Default::default(),
                    tx_bytes: Default::default(),
                    rx_packets: 0,
                    tx_packets: 0,
                    rx_errors: 0,
                    tx_errors: 0,
                });
        all_rx += network_stats.rx_bytes.0;
        all_tx += network_stats.tx_bytes.0;
    }

    let tmp_tx = unsafe { TMP_TX };
    let tmp_rx = unsafe { TMP_RX };

    unsafe {
        TMP_TX = all_tx;
        TMP_RX = all_rx;
    }

    Network {
        up: all_rx - tmp_rx,
        down: all_tx - tmp_tx,
        total_up: all_rx,
        total_down: all_tx,
    }
}

pub fn realtime_connections(systemstat_sys: &systemstat::System) -> Connections {
    let socket = if let Ok(socket) = systemstat_sys.socket_stats() {
        socket
    } else {
        return Connections { tcp: 0, udp: 0 };
    };

    let tcp = socket.tcp_sockets_in_use + socket.tcp6_sockets_in_use;
    let udp = socket.udp_sockets_in_use + socket.udp6_sockets_in_use;
    Connections {
        tcp: tcp as u64,
        udp: udp as u64,
    }
}

pub fn realtime_uptime(systemstat_sys: &systemstat::System) -> u64 {
    systemstat_sys
        .uptime()
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

pub fn realtime_process(sysinfo_sys: &System) -> u64 {
    sysinfo_sys.processes().len() as u64
}
