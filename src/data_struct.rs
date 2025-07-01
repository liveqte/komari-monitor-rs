use crate::get_info::{
    arch, cpu_info_without_usage, ip, mem_info_without_usage, os, realtime_connections,
    realtime_cpu, realtime_disk, realtime_load, realtime_mem, realtime_network, realtime_process,
    realtime_swap, realtime_uptime,
};
use log::debug;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicInfo {
    pub arch: String,
    pub cpu_cores: u16,
    pub cpu_name: String,
    pub gpu_name: String, // 暂不支持

    pub disk_total: u64,
    pub swap_total: u64,
    pub mem_total: u64,

    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>,

    pub os: String,
    pub version: String,
    pub virtualization: String,
}

impl BasicInfo {
    pub async fn build(sysinfo_sys: &sysinfo::System, reqwest_client: &Client) -> Self {
        let cpu = cpu_info_without_usage(&sysinfo_sys);
        let mem_disk = mem_info_without_usage(&sysinfo_sys);
        let ip = ip(reqwest_client).await;
        let os = os().await;
        Self {
            arch: arch(),
            cpu_cores: cpu.cores,
            cpu_name: cpu.name,

            gpu_name: "".to_string(),
            disk_total: mem_disk.disk_total,
            swap_total: mem_disk.swap_total,
            mem_total: mem_disk.mem_total,
            ipv4: ip.ipv4,
            ipv6: ip.ipv6,
            os: format!("{} {}", os.os, os.version),
            version: format!("komari-monitor-rs {}", env!("CARGO_PKG_VERSION")),
            virtualization: os.virtualization,
        }
    }

    pub async fn push(
        sysinfo_sys: &sysinfo::System,
        reqwest_client: &Client,
        basic_info_url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let basic_info = BasicInfo::build(&sysinfo_sys, &reqwest_client).await;
        debug!("Basic Info: {:?}", basic_info);
        let status = reqwest_client
            .post(basic_info_url)
            .json(&basic_info)
            .send()
            .await?
            .status()
            .is_success();

        if status {
            Ok(())
        } else {
            Err("上传 Basic Info 失败".into())
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CPU {
    pub usage: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RAM {
    pub total: u64,
    pub used: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Swap {
    pub total: u64,
    pub used: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Disk {
    pub total: u64,
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
    pub async fn build(sysinfo_sys: &sysinfo::System, systemstat_sys: &systemstat::System) -> Self {
        Self {
            cpu: realtime_cpu(sysinfo_sys),
            ram: realtime_mem(sysinfo_sys),
            swap: realtime_swap(sysinfo_sys),
            disk: realtime_disk(),
            load: realtime_load(systemstat_sys),
            network: realtime_network(systemstat_sys),
            connections: realtime_connections(systemstat_sys),
            uptime: realtime_uptime(systemstat_sys),
            process: realtime_process(sysinfo_sys),
            message: "".to_string(),
        }
    }
}
