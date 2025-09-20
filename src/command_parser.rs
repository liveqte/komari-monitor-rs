use std::fmt::{Display, Formatter};
use crate::get_info::DURATION;
use crate::rustls_config::create_dangerous_config;
use clap::{Parser, ValueEnum};
use std::fs;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::time::{Duration, timeout};
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

    /// 公网 IP 接口
    #[arg(long, default_value_t=ip_provider())]
    pub ip_provider: IpProvider,

    /// 启用 Terminal (默认关闭)
    #[arg(long, default_value_t = false)]
    pub terminal: bool,

    /// 自定义 Terminal 入口
    #[arg(long, default_value_t = terminal_entry())]
    pub terminal_entry: String,

    /// 设置虚假倍率
    #[arg(short, long, default_value_t = 1.0)]
    pub fake: f64,

    /// 设置 Real-Time Info 上传间隔时间 (ms)
    #[arg(long, default_value_t = 1000)]
    pub realtime_info_interval: u64,

    /// 启用 TLS (默认关闭)
    #[arg(long, default_value_t = false)]
    pub tls: bool,

    /// 忽略证书验证
    #[arg(long, default_value_t = false)]
    pub ignore_unsafe_cert: bool,
}

fn terminal_entry() -> String {
    "default".to_string()
}

fn ip_provider() -> IpProvider {
    IpProvider::Ipinfo
}

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum IpProvider {
    Cloudflare,
    Ipinfo,
}

impl Display for IpProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IpProvider::Cloudflare => write!(f, "cloudflare"),
            IpProvider::Ipinfo => write!(f, "ipinfo"),
        }
    }
}

impl Args {
    pub fn par() -> Self {
        let mut args = Self::parse();
        unsafe {
            DURATION = args.realtime_info_interval as f64;
        }
        if args.terminal_entry == "default" {
            args.terminal_entry = {
                if cfg!(windows) {
                    "cmd.exe".to_string()
                } else if fs::exists("/bin/bash").unwrap_or(false) {
                    "bash".to_string()
                } else {
                    "sh".to_string()
                }
            };
        }
        args
    }
}

pub async fn connect_ws(
    url: &str,
    tls: bool,
    skip_verify: bool,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, String> {
    let connection_timeout = Duration::from_secs(10);

    if tls {
        if skip_verify {
            timeout(
                connection_timeout,
                connect_async_tls_with_config(
                    url,
                    None,
                    false,
                    Some(Connector::Rustls(Arc::new(create_dangerous_config()))),
                ),
            )
            .await
            .map_err(|_| "WebSocket 连接超时".to_string())?
            .map(|ws| ws.0)
            .map_err(|_| "无法创立 WebSocket 连接".to_string())
        } else {
            timeout(
                connection_timeout,
                connect_async_tls_with_config(url, None, false, None),
            )
            .await
            .map_err(|_| "WebSocket 连接超时".to_string())?
            .map(|ws| ws.0)
            .map_err(|_| "无法创立 WebSocket 连接".into())
        }
    } else {
        timeout(connection_timeout, connect_async(url))
            .await
            .map_err(|_| "WebSocket 连接超时".to_string())?
            .map(|ws| ws.0)
            .map_err(|_| "无法创立 WebSocket 连接".to_string())
    }
}
