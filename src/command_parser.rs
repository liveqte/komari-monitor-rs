use crate::rustls_config::create_dangerous_config;
use clap::Parser;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async, connect_async_tls_with_config,
};

/// Komari Monitor Agent
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// 设置主端 Http 地址
    #[arg(long)]
    pub http_server: String,

    /// 设置主端 WebSocket 地址
    #[arg(long)]
    pub ws_server: String,

    /// 设置 Token
    #[arg(short, long)]
    pub token: String,

    /// 设置 Real-Time Info 上传间隔时间 (sec)
    #[arg(long, default_value_t = 1)]
    pub realtime_info_interval: u64,

    /// 启用 TLS (默认关闭)
    #[arg(long, default_value_t = false)]
    pub tls: bool,

    /// 忽略证书验证
    #[arg(long, default_value_t = false)]
    pub ignore_unsafe_cert: bool,
}

impl Args {
    pub fn par() -> Self {
        Self::parse()
    }
}

pub async fn connect_ws(
    url: &str,
    tls: bool,
    skip_verify: bool,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
    if tls {
        if !skip_verify {
            if let Ok(ws) = connect_async_tls_with_config(url, None, false, None).await {
                Ok(ws.0)
            } else {
                Err("无法创立 WebSocket 连接".into())
            }
        } else if let Ok(ws) = connect_async_tls_with_config(
            url,
            None,
            false,
            Some(Connector::Rustls(Arc::new(create_dangerous_config()))),
        )
        .await
        {
            Ok(ws.0)
        } else {
            Err("无法创立 WebSocket 连接".to_string())
        }
    } else if let Ok(ws) = connect_async(url).await {
        Ok(ws.0)
    } else {
        Err("无法创立 WebSocket 连接".to_string())
    }
}
