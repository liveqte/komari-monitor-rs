use crate::data_struct::Load;
use log::trace;

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
