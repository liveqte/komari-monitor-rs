// #![warn(clippy::all, clippy::pedantic)]

use crate::command_parser::{Args, LogLevel};
use crate::data_struct::{BasicInfo, RealTimeInfo};
use crate::exec::exec_command;
use crate::ping::ping_target;
use crate::pty::{get_pty_ws_link, handle_pty_session};
use crate::utils::{build_urls, connect_ws, init_logger};
use futures::{SinkExt, StreamExt};
use log::{info, Level};
use miniserde::{Deserialize, Serialize, json};
use std::sync::Arc;
use std::time::Duration;
use futures::stream::{SplitSink, SplitStream};
use sysinfo::{CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

mod command_parser;
mod data_struct;
mod exec;
mod get_info;
mod ping;
mod pty;
mod rustls_config;

#[cfg(target_os = "linux")]
mod netlink;
mod utils;
mod callbacks;

#[tokio::main]
async fn main() {
    let args = Args::par();

    init_logger(&args.log_level);

    let connection_urls = build_urls(&args.http_server, &args.ws_server, &args.token).unwrap();

    info!("成功读取参数: {args:?}");

    loop {
        let Ok(ws_stream) = connect_ws(&real_time_url, args.tls, args.ignore_unsafe_cert).await
        else {
            eprintln!("无法连接到 Websocket 服务器，5 秒后重新尝试");
            sleep(Duration::from_secs(5)).await;
            continue;
        };

        let (write, mut read): (SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>, SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>) = ws_stream.split();

        let locked_write: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>> = Arc::new(Mutex::new(write));

        let _listener = tokio::spawn({
            let exec_callback_url = exec_callback_url.clone();
            let locked_write = locked_write.clone();
            let args = args.clone();
            async move {

            }
        });

        let mut sysinfo_sys = sysinfo::System::new();
        let mut networks = Networks::new_with_refreshed_list();
        let mut disks = Disks::new();
        sysinfo_sys.refresh_cpu_list(
            CpuRefreshKind::nothing()
                .without_cpu_usage()
                .without_frequency(),
        );
        sysinfo_sys.refresh_memory_specifics(MemoryRefreshKind::everything());

        let basic_info = BasicInfo::build(&sysinfo_sys, args.fake, &args.ip_provider).await;
        println!("{basic_info:?}");

        if let Err(e) = basic_info
            .push(&basic_info_url, args.ignore_unsafe_cert)
            .await
        {
            eprintln!("推送 Basic Info 时发生错误: {e}");
        } else {
            println!("推送 Basic Info 成功");
        }

        loop {
            let start_time = tokio::time::Instant::now();
            sysinfo_sys.refresh_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::everything().without_frequency())
                    .with_memory(MemoryRefreshKind::everything()),
            );
            networks.refresh(true);
            disks.refresh_specifics(true, DiskRefreshKind::nothing().with_storage());
            let real_time = RealTimeInfo::build(&sysinfo_sys, &networks, &disks, args.fake);

            let json = json::to_string(&real_time);
            {
                let mut write = locked_write.lock().await;
                if let Err(e) = write.send(Message::Text(Utf8Bytes::from(json))).await {
                    eprintln!("推送 RealTime 时发生错误，尝试重新连接: {e}");
                    break;
                }
            }
            let end_time = start_time.elapsed();

            sleep(Duration::from_millis({
                let end = u64::try_from(end_time.as_millis()).unwrap_or(0);
                args.realtime_info_interval.saturating_sub(end)
            }))
            .await;
        }
    }
}
