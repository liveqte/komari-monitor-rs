use crate::command_parser::{Args, connect_ws};
use crate::data_struct::{BasicInfo, RealTimeInfo};
use futures::{SinkExt, StreamExt};
use log::{debug, error, info};
use std::process::exit;
use std::time::Duration;
use sysinfo::CpuRefreshKind;
use systemstat::Platform;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};

mod command_parser;
mod data_struct;
mod get_info;
mod rustls_config;

#[tokio::main]
async fn main() {
    simple_logger::SimpleLogger::new()
        .with_colors(true)
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();
    let args = Args::par();
    let basic_info_url = format!(
        "{}/api/clients/uploadBasicInfo?token={}",
        args.http_server, args.token
    );
    let real_time_url = format!("{}/api/clients/report?token={}", args.ws_server, args.token);

    info!("成功读取参数: {:?}", args);

    let reqwest_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("curl/11.45.14");

    let reqwest_client = match if args.ignore_unsafe_cert {
        reqwest_client.danger_accept_invalid_certs(true).build()
    } else {
        reqwest_client.build()
    } {
        Ok(client) => client,
        Err(e) => {
            error!("无法初始化客户端: {}", e);
            exit(1);
        }
    };

    let _basic_info = tokio::spawn(async move {
        let mut sysinfo_sys = sysinfo::System::new();
        sysinfo_sys.refresh_cpu_specifics(CpuRefreshKind::nothing().without_cpu_usage());
        sysinfo_sys.refresh_memory();
        loop {
            sysinfo_sys.refresh_cpu_specifics(CpuRefreshKind::nothing().without_cpu_usage());
            sysinfo_sys.refresh_memory();
            if let Err(e) = BasicInfo::push(&sysinfo_sys, &reqwest_client, &basic_info_url).await {
                error!("推送 Basic 时发生错误: {}", e);
            }
            sleep(Duration::from_secs(args.basic_info_interval)).await;
        }
    });

    loop {
        let ws_stream =
            if let Ok(ws) = connect_ws(&real_time_url, args.tls, args.ignore_unsafe_cert).await {
                ws
            } else {
                error!("无法连接到 Websocket 服务器，5 秒后重新尝试");
                sleep(Duration::from_secs(5)).await;
                continue;
            };

        let (mut write, mut _read) = ws_stream.split();

        let mut sysinfo_sys = sysinfo::System::new();
        let systemstat_sys = systemstat::System::new();
        sysinfo_sys.refresh_all();

        loop {
            sleep(Duration::from_secs(args.realtime_info_interval)).await;
            sysinfo_sys.refresh_all();
            let real_time = RealTimeInfo::build(&sysinfo_sys, &systemstat_sys).await;
            let json = serde_json::to_string(&real_time).unwrap();
            debug!("RealTime: {}", json);
            if let Err(e) = write.send(Message::Text(Utf8Bytes::from(json))).await {
                error!("推送 RealTime 时发生错误，尝试重新连接: {}", e);
                break;
            };
        }
    }
}
