// #![warn(clippy::all, clippy::pedantic)]

use crate::command_parser::{connect_ws, Args};
use crate::data_struct::{BasicInfo, RealTimeInfo};
use crate::exec::exec_command;
use crate::ping::ping_target;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use miniserde::{json, Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
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
mod rustls_config;

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
        let ws_stream =
            if let Ok(ws) = connect_ws(&real_time_url, args.tls, args.ignore_unsafe_cert).await {
                ws
            } else {
                eprintln!("无法连接到 Websocket 服务器，5 秒后重新尝试");
                sleep(Duration::from_secs(5)).await;
                continue;
            };

        let (write, mut read): (
            SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
            SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        ) = ws_stream.split();

        let locked_write = Arc::new(Mutex::new(write));

        let _listener = tokio::spawn({
            let exec_callback_url = exec_callback_url.clone();
            let locked_write = locked_write.clone();
            async move {
                while let Some(msg) = read.next().await {
                    let msg = if let Ok(msg) = msg {
                        msg
                    } else {
                        continue;
                    };

                    let utf8 = if let Ok(utf8) = msg.into_text() {
                        utf8
                    } else {
                        continue;
                    };

                    println!("主端传入信息: {}", utf8.as_str());

                    #[derive(Serialize, Deserialize)]
                    struct Message {
                        message: String,
                    }

                    let json: Message = if let Ok(value) = json::from_str(utf8.as_str()) {
                        value
                    } else {
                        continue;
                    };

                    match json.message.as_str() {
                        "exec" => {
                            if let Err(e) = {
                                exec_command(
                                    {
                                        if let Ok(parsed) = json::from_str(utf8.as_str()) {
                                            parsed
                                        } else {
                                            continue;
                                        }
                                    },
                                    &exec_callback_url,
                                )
                                .await
                            } {
                                eprintln!("Exec Error: {e}");
                            }
                        }
                        "ping" => {
                            let parsed = if let Ok(parsed) = json::from_str(utf8.as_str()) {
                                parsed
                            } else {
                                continue;
                            };

                            match ping_target(parsed).await {
                                Ok(json) => {
                                    let mut write = locked_write.lock().await;
                                    println!("Ping Success: {}", json::to_string(&json));
                                    write
                                        .send(tokio_tungstenite::tungstenite::Message::Text(
                                            Utf8Bytes::from(json::to_string(&json)),
                                        ))
                                        .await
                                        .unwrap();
                                }
                                Err(err) => {
                                    eprintln!("Ping Error: {err}");
                                }
                            }
                        }
                        _ => continue,
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

        if let Err(e) = BasicInfo::push(&sysinfo_sys, &basic_info_url).await {
            eprintln!("推送 Basic Info 时发生错误: {e}");
        }

        loop {
            sysinfo_sys.refresh_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::everything().without_frequency())
                    .with_memory(MemoryRefreshKind::everything()),
            );
            networks.refresh(true);
            disks.refresh_specifics(true, DiskRefreshKind::nothing().with_storage());
            let real_time = RealTimeInfo::build(&sysinfo_sys, &networks, &disks).await;

            let json = json::to_string(&real_time);
            let mut write = locked_write.lock().await;
            if let Err(e) = write.send(Message::Text(Utf8Bytes::from(json))).await {
                eprintln!("推送 RealTime 时发生错误，尝试重新连接: {e}");
                break;
            }

            sleep(Duration::from_secs(args.realtime_info_interval)).await;
        }
    }
}
