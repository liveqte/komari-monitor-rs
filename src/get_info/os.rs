use log::trace;
use sysinfo::System;

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
