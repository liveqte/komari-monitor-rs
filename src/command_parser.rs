use clap::{Parser, ValueEnum}; 
use std::fs;

#[derive(Parser, Debug, Clone)]
#[command(
    version,
    long_about = "komari-monitor-rs is a third-party high-performance monitoring agent for the komari monitoring service."
)]
pub struct Args {
    #[arg(long)]
    #[arg(env = "HTTP_SERVER")]
    pub http_server: String,
    
    #[arg(long)]
    #[arg(env = "WS_SERVER")]
    pub ws_server: Option<String>,
    
    #[arg(short, long, allow_hyphen_values = true)]
    #[arg(env = "TOKEN")]
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

#[derive(Debug, Clone, ValueEnum)]
pub enum IpProvider {
    Cloudflare,
    Ipinfo,
}

fn log_level() -> LogLevel {
    LogLevel::Info
}

#[derive(Debug, Clone, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Args {
    pub fn par() -> Self {
        let mut args = Self::parse();
        unsafe {
            crate::get_info::network::DURATION = args.realtime_info_interval as f64;
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
