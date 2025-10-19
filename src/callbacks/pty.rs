use futures::{SinkExt, StreamExt};
use log::{error, info};
use miniserde::{Deserialize, Serialize};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::{sync::mpsc, task};
use tokio_tungstenite::tungstenite::Bytes;
use tokio_tungstenite::{WebSocketStream, tungstenite::protocol::Message};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TerminalEvent {
    message: String,
    request_id: String,
}

pub fn get_pty_ws_link(utf8_str: &str, ws_terminal_url: &str) -> Result<String, String> {
    let ping_event: TerminalEvent =
        miniserde::json::from_str(utf8_str).map_err(|_| "无法解析 TerminalEvent".to_string())?;

    Ok(format!(
        "{ws_terminal_url}&id={request_id}",
        request_id = ping_event.request_id
    ))
}

pub async fn handle_pty_session<S>(ws_stream: WebSocketStream<S>, cmd: &str) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let pty_system = NativePtySystem::default();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("无法创建 PTY: {e}"))?;

    let mut cmd = CommandBuilder::new(cmd);

    if !cfg!(windows) {
        cmd.env("TERM", "xterm-256color");
        cmd.env("LANG", "C.UTF-8");
        cmd.env("LC_ALL", "C.UTF-8");
    }

    let mut pty_reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("无法获取 PTY Reader: {e}"))?;
    let pty_writer = Arc::new(Mutex::new(
        pair.master
            .take_writer()
            .map_err(|e| format!("无法获取 PTY Writer: {e}"))?,
    ));

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("无法启动进程: {e}"))?;

    info!("在 PTY 中启动了终端, PID: {:?}", child.process_id());

    let (ws_sender, mut ws_receiver) = ws_stream.split();
    let (pty_to_ws_tx, mut pty_to_ws_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    task::spawn_blocking(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match pty_reader.read(&mut buffer) {
                Ok(count) if count > 0 => {
                    if pty_to_ws_tx.send(buffer[..count].to_vec()).is_err() {
                        info!("PTY reader: WebSocket端已关闭，停止读取。");
                        break;
                    }
                }
                Ok(_) | Err(_) => {
                    info!("PTY reader: PTY 已关闭，停止读取。");
                    break;
                }
            }
        }
    });

    let pty_to_ws_task = tokio::spawn(async move {
        let mut ws_sender = ws_sender;
        while let Some(data) = pty_to_ws_rx.recv().await {
            if ws_sender
                .send(Message::Binary(Bytes::from(data)))
                .await
                .is_err()
            {
                error!("发送数据到 WebSocket 失败");
                break;
            }
        }
    });

    let ws_to_pty_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(msg) => match handle_ws_message(msg, &pty_writer) {
                    Err(e) => {
                        error!("处理 WebSocket 消息失败: {e}");
                        break;
                    }
                    Ok(Some(resize)) => {
                        if let Err(e) = pair.master.resize(PtySize {
                            rows: resize.rows,
                            cols: resize.cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        }) {
                            error!("无法调整 PTY 大小: {e}");
                        }
                    }
                    _ => {}
                },
                Err(e) => {
                    error!("从 WebSocket 接收消息时出错: {e}");
                    break;
                }
            }
        }
    });

    tokio::select! {
        _ = pty_to_ws_task => info!("PTY -> WebSocket 任务结束。"),
        _ = ws_to_pty_task => info!("WebSocket -> PTY 任务结束。"),
    }

    info!("正在关闭会话，终止子进程...");
    if let Err(e) = child.kill() {
        error!("终止子进程失败: {e}");
    }
    child.wait().map_err(|e| format!("无法终止子线程: {e}"))?;
    info!("会话已成功关闭。");

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct NeedResize {
    #[serde(rename = "type")]
    type_str: String,
    cols: u16,
    rows: u16,
}

fn handle_ws_message(
    msg: Message,
    pty_writer: &Arc<Mutex<Box<dyn Write + Send>>>,
) -> Result<Option<NeedResize>, String> {
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct HeartBeat {
        #[serde(rename = "type")]
        type_str: String,
        timestamp: String,
    }

    match msg {
        Message::Text(text) => {
            if miniserde::json::from_str::<HeartBeat>(text.as_ref()).is_ok() {
                return Ok(None);
            }
            if let Ok(resize) = miniserde::json::from_str::<NeedResize>(text.as_ref()) {
                return Ok(Some(resize));
            }
            pty_writer
                .lock()
                .unwrap()
                .write_all(text.as_bytes())
                .map_err(|e| format!("无法写入 PTY: {e}"))?;
        }
        Message::Binary(data) => {
            pty_writer
                .lock()
                .unwrap()
                .write_all(&data)
                .map_err(|e| format!("无法写入 PTY: {e}"))?;
        }
        Message::Close(_) => {
            return Err(String::from("WebSocket 连接已关闭"));
        }
        _ => {}
    }
    Ok(None)
}
