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
mod utils;

#[tokio::main]
async fn main() {
    let args = Args::par();

    init_logger(&args.log_level);
    
    let (basic_info_url, real_time_url, exec_callback_url) = build_urls(&args.http_server, &args.ws_server, &args.token).unwrap();

    info!("成功读取参数: {args:?}");

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

                    let utf8_cloned = utf8.clone();
                    let args_cloned = args.clone();

                    match json.message.as_str() {
                        "exec" => {
                            let exec_callback_url_for_task = exec_callback_url.clone();
                            tokio::spawn(async move {
                                if let Err(e) = exec_command(
                                    &utf8_cloned,
                                    &exec_callback_url_for_task,
                                    args.ignore_unsafe_cert,
                                )
                                .await
                                {
                                    eprintln!("Exec Error: {e}");
                                }
                            });
                        }

                        "ping" => {
                            let locked_write_for_ping = locked_write.clone();
                            tokio::spawn(async move {
                                match ping_target(&utf8_cloned).await {
                                    Ok(json_res) => {
                                        let mut write = locked_write_for_ping.lock().await;
                                        println!("Ping Success: {}", json::to_string(&json_res));
                                        if let Err(e) = write
                                            .send(Message::Text(Utf8Bytes::from(json::to_string(
                                                &json_res,
                                            ))))
                                            .await
                                        {
                                            eprintln!(
                                                "推送 ping result 时发生错误，尝试重新连接: {e}"
                                            );
                                        }
                                    }
                                    Err(err) => {
                                        eprintln!("Ping Error: {err}");
                                    }
                                }
                            });
                        }

                        "terminal" => {
                            if args.terminal {
                                tokio::spawn(async move {
                                    let ws_url = match get_pty_ws_link(
                                        &utf8_cloned,
                                        &args_cloned.ws_server,
                                        &args_cloned.token.clone(),
                                    ) {
                                        Ok(ws_url) => ws_url,
                                        Err(e) => {
                                            eprintln!("无法获取 PTY Websocket URL: {e}");
                                            return;
                                        }
                                    };

                                    let ws_stream = match connect_ws(
                                        &ws_url,
                                        args.tls,
                                        args.ignore_unsafe_cert,
                                    )
                                    .await
                                    {
                                        Ok(ws_stream) => ws_stream,
                                        Err(e) => {
                                            eprintln!("无法连接到 PTY Websocket: {e}");
                                            return;
                                        }
                                    };

                                    if let Err(e) = handle_pty_session(
                                        ws_stream,
                                        args_cloned.terminal_entry.clone(),
                                    )
                                    .await
                                    {
                                        eprintln!("PTY Websocket 处理错误: {e}");
                                    }
                                });
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
