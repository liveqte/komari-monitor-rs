use crate::get_info::DURATION;
use clap::{Parser, ValueEnum};
use std::fmt::{Display, Formatter};
use std::fs;

/// Komari Monitor Agent
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// 设置主端 Http 地址
    #[arg(long)]
    pub http_server: String,

    /// 设置主端 WebSocket 地址
    #[arg(long)]
    pub ws_server: Option<String>,

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

    /// 设置日志等级 (反馈问题请开启 Debug 或者 Trace)
    #[arg(long, default_value_t = log_level())]
    pub log_level: LogLevel,
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

fn log_level() -> LogLevel {
    LogLevel::Info
}

#[derive(Parser, Debug, Clone, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LogLevel::Error => "error",
                LogLevel::Warn => "warn",
                LogLevel::Info => "info",
                LogLevel::Debug => "debug",
                LogLevel::Trace => "trace",
            }
        )
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

