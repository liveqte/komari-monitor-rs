use crate::data_struct::Cpu;
use log::trace;
use std::collections::HashSet;
use sysinfo::System;

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
