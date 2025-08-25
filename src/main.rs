#![warn(clippy::all, clippy::pedantic)]

use crate::command_parser::{connect_ws, Args};
use crate::data_struct::{BasicInfo, RealTimeInfo};
use crate::exec::exec_command;
use crate::ping::ping_target;
use crate::pty::{get_pty_ws_link, handle_pty_session};
use futures::{SinkExt, StreamExt};
use miniserde::{json, Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};

mod command_parser;
mod data_struct;
mod exec;
mod get_info;
mod ping;
mod pty;
mod rustls_config;

#[cfg(target_os = "linux")]
mod netlink;

#[tokio::main]
async fn main() {
    let args = Args::par();
    let basic_info_url = format!(
        "{}/api/clients/uploadBasicInfo?token={}",
        args.http_server, args.token
    );
    let real_time_url = format!("{}/api/clients/report?token={}", args.ws_server, args.token);
    let exec_callback_url = format!(
        "{}/api/clients/task/result?token={}",
        args.http_server, args.token
    );

    println!("成功读取参数: {args:?}");

    loop {
        let Ok(ws_stream) = connect_ws(&real_time_url, args.tls, args.ignore_unsafe_cert).await
        else {
            eprintln!("无法连接到 Websocket 服务器，5 秒后重新尝试");
            sleep(Duration::from_secs(5)).await;
            continue;
        };

        let (write, mut read) = ws_stream.split();

        let locked_write = Arc::new(Mutex::new(write));

        let _listener = tokio::spawn({
            let exec_callback_url = exec_callback_url.clone();
            let locked_write = locked_write.clone();
            let args = args.clone();
            async move {
                while let Some(msg) = read.next().await {
                    let Ok(msg) = msg else {
                        continue;
                    };

                    let Ok(utf8) = msg.into_text() else {
                        continue;
                    };

                    println!("主端传入信息: {}", utf8.as_str());

                    #[derive(Serialize, Deserialize)]
                    struct Msg {
                        message: String,
                    }

                    let json: Msg = if let Ok(value) = json::from_str(utf8.as_str()) {
                        value
                    } else {
                        continue;
                    };

                    match json.message.as_str() {
                        "exec" => {
                            if let Err(e) = {
                                exec_command(
                                    utf8.as_str(), // 修复语法错误
                                    &exec_callback_url,
                                    args.ignore_unsafe_cert,
                                )
                                .await
                            } {
                                eprintln!("Exec Error: {e}");
                            }
                        }
                        "ping" => {
                            let utf8_str = utf8.as_str();
                            match ping_target(utf8_str).await {
                                Ok(json) => {
                                    let mut write = locked_write.lock().await;
                                    println!("Ping Success: {}", json::to_string(&json));
                                    if let Err(e) = write
                                        .send(Message::Text(Utf8Bytes::from(json::to_string(
                                            &json,
                                        ))))
                                        .await
                                    {
                                        eprintln!("推送 ping result 时发生错误，尝试重新连接: {e}");
                                    }
                                }
                                Err(err) => {
                                    eprintln!("Ping Error: {err}");
                                }
                            }
                        }
                        "terminal" => {
                            if args.terminal {
                                let ws_url = match get_pty_ws_link(
                                    utf8.as_str(),
                                    &args.ws_server.clone(),
                                    &args.token.clone(),
                                ) {
                                    Ok(ws_url) => ws_url,
                                    Err(e) => {
                                        eprintln!("无法获取 PTY Websocket URL: {e}");
                                        continue;
                                    }
                                };

                                let ws_stream = match connect_ws(
                                    ws_url.as_str(),
                                    args.tls,
                                    args.ignore_unsafe_cert,
                                )
                                .await
                                {
                                    Ok(ws_stream) => ws_stream,
                                    Err(e) => {
                                        eprintln!("无法连接到 PTY Websocket: {e}");
                                        continue;
                                    }
                                };

                                if let Err(e) =
                                    handle_pty_session(ws_stream, args.terminal_entry.clone()).await
                                {
                                    eprintln!("PTY Websocket 处理错误: {e}");
                                }
                            } else {
                                eprintln!("终端功能未启用");
                            }
                        }
                        _ => {}
                    }
                }
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

        if let Err(e) = BasicInfo::push(
            &sysinfo_sys,
            &basic_info_url,
            args.fake,
            args.ignore_unsafe_cert,
        )
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
