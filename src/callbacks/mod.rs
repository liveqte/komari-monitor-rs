use std::sync::Arc;
use futures::{SinkExt, StreamExt};
use futures::stream::{SplitSink, SplitStream};
use log::{error, info};
use miniserde::{json, Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use crate::callbacks::exec::exec_command;
use crate::callbacks::ping::ping_target;
use crate::callbacks::pty::{get_pty_ws_link, handle_pty_session};
use crate::command_parser::Args;
use crate::utils::{connect_ws, ConnectionUrls};

pub mod exec;
pub mod ping;
pub mod pty;

#[derive(Serialize, Deserialize)]
struct Msg {
    message: String,
}

pub async fn handle_callbacks(args: &Args, connection_urls: &ConnectionUrls, reader: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, locked_writer: &Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>) -> () {
    while let Some(msg) = reader.next().await {
        let Ok(msg) = msg else {
            continue;
        };

        let Ok(utf8) = msg.into_text() else {
            continue;
        };

        info!("主端传入信息: {}", utf8.as_str());

        let json: Msg = if let Ok(value) = json::from_str(utf8.as_str()) {
            value
        } else {
            continue;
        };

        let utf8_cloned = utf8.clone();

        match json.message.as_str() {
            "exec" => {
                tokio::spawn({
                    let utf8_cloned_for_exec = utf8_cloned.clone();
                    let exec_callback_url = connection_urls.exec_callback_url.clone();
                    let ignore_unsafe_cert = args.ignore_unsafe_cert.clone();

                    async move {
                        if let Err(e) = exec_command(
                            &utf8_cloned_for_exec,
                            &exec_callback_url,
                            &ignore_unsafe_cert,
                        )
                            .await
                        {
                            error!("Exec Error: {e}");
                        }
                    }
                });
            }

            "ping" => {
                let locked_write_for_ping = locked_writer.clone();
                tokio::spawn(async move {
                    match ping_target(&utf8_cloned).await {
                        Ok(json_res) => {
                            let mut write = locked_write_for_ping.lock().await;
                            info!("Ping Success: {}", json::to_string(&json_res));
                            if let Err(e) = write
                                .send(Message::Text(Utf8Bytes::from(json::to_string(
                                    &json_res,
                                ))))
                                .await
                            {
                                error!(
                                    "推送 ping result 时发生错误，尝试重新连接: {e}"
                                );
                            }
                        }
                        Err(err) => {
                            error!("Ping Error: {err}");
                        }
                    }
                });
            }

            "terminal" => {
                if args.terminal {
                    let ws_url_base = connection_urls.clone().ws_url_base.clone();
                    let args = args.clone();
                    let utf8_cloned = utf8_cloned.clone();

                    tokio::spawn(async move {
                        let ws_url = match get_pty_ws_link(
                            &utf8_cloned,
                            &ws_url_base,
                            &args.token,
                        ) {
                            Ok(ws_url) => ws_url,
                            Err(e) => {
                                error!("无法获取 PTY Websocket URL: {e}");
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
                                error!("无法连接到 PTY Websocket: {e}");
                                return;
                            }
                        };

                        if let Err(e) = handle_pty_session(
                            ws_stream,
                            &args.terminal_entry,
                        )
                            .await
                        {
                            error!("PTY Websocket 处理错误: {e}");
                        }
                    });
                } else {
                    error!("终端功能未启用");
                }
            }
            _ => {}
        }
    }
}