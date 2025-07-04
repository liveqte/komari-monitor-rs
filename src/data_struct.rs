use crate::get_info::{
    arch, cpu_info_without_usage, ip, mem_info_without_usage, os, realtime_connections,
    realtime_cpu, realtime_disk, realtime_load, realtime_mem, realtime_network, realtime_process,
    realtime_swap, realtime_uptime,
};
use miniserde::{Deserialize, Serialize};
use sysinfo::{Disks, Networks};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicInfo {
    pub arch: String,
    pub cpu_cores: u16,
    pub cpu_name: String,
    pub gpu_name: String, // 暂不支持

    pub disk_total: u64,
    pub swap_total: u64,
    pub mem_total: u64,

    pub ipv4: Option<String>,
    pub ipv6: Option<String>,

    pub os: String,
    pub version: String,
    pub virtualization: String,
}

impl BasicInfo {
    pub async fn build(sysinfo_sys: &sysinfo::System) -> Self {
        let cpu = cpu_info_without_usage(sysinfo_sys);
        let mem_disk = mem_info_without_usage(sysinfo_sys);
        let ip = ip().await;
        let os = os().await;
        Self {
            arch: arch(),
            cpu_cores: cpu.cores,
            cpu_name: cpu.name,

            gpu_name: String::new(),
            disk_total: mem_disk.disk_total,
            swap_total: mem_disk.swap_total,
            mem_total: mem_disk.mem_total,
            ipv4: ip.ipv4.map(|ip| ip.to_string()),
            ipv6: ip.ipv6.map(|ip| ip.to_string()),
            os: format!("{} {}", os.os, os.version),
            version: format!("komari-monitor-rs {}", env!("CARGO_PKG_VERSION")),
            virtualization: os.virtualization,
        }
    }

    pub async fn push(
        sysinfo_sys: &sysinfo::System,
        basic_info_url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let basic_info = Self::build(sysinfo_sys).await;

        let resp = if let Ok(resp) = ureq::post(basic_info_url)
            .header("User-Agent", "curl/11.45.14-rs")
            .send(&miniserde::json::to_string(&basic_info))
        {
            println!("{:?}", resp.body().charset());
            resp
        } else {
            return Err(Box::new(std::io::Error::other(
                "推送 Basic Info Post 时发生错误",
            )));
        };

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Box::new(std::io::Error::other(
                "推送 Basic Info 时发生错误",
            )))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CPU {
    pub usage: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RAM {
    pub used: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Swap {
    pub used: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Disk {
    pub used: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Load {
    pub load1: f64,
    pub load5: f64,
    pub load15: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Network {
    pub up: u64,
    pub down: u64,
    #[serde(rename = "totalUp")]
    pub total_up: u64,

    #[serde(rename = "totalDown")]
    pub total_down: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Connections {
    pub tcp: u64,
    pub udp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RealTimeInfo {
    pub cpu: CPU,
    pub ram: RAM,
    pub swap: Swap,
    pub disk: Disk,
    pub load: Load,
    pub network: Network,
    pub connections: Connections,
    pub uptime: u64,
    pub process: u64,
    pub message: String,
}

impl RealTimeInfo {
    pub async fn build(sysinfo_sys: &sysinfo::System, network: &Networks, disk: &Disks) -> Self {
        Self {
            cpu: realtime_cpu(sysinfo_sys),
            ram: realtime_mem(sysinfo_sys),
            swap: realtime_swap(sysinfo_sys),
            disk: realtime_disk(disk),
            load: realtime_load(),
            network: realtime_network(network),
            connections: realtime_connections(),
            uptime: realtime_uptime(),
            process: realtime_process(),
            message: String::new(),
        }
    }
}
