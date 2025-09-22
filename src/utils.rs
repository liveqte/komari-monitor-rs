use crate::command_parser::LogLevel;
use crate::rustls_config::create_dangerous_config;
use log::{Level, info};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async, connect_async_tls_with_config,
};
use url::ParseError;

pub fn init_logger(log_level: &LogLevel) {
    if cfg!(target_os = "windows") {
        simple_logger::set_up_windows_color_terminal();
    }

    match log_level {
        LogLevel::Error => simple_logger::init_with_level(Level::Error).unwrap(),
        LogLevel::Warn => simple_logger::init_with_level(Level::Warn).unwrap(),
        LogLevel::Info => simple_logger::init_with_level(Level::Info).unwrap(),
        LogLevel::Debug => simple_logger::init_with_level(Level::Debug).unwrap(),
        LogLevel::Trace => simple_logger::init_with_level(Level::Trace).unwrap(),
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionUrls {
    pub http_url_base: String,
    pub ws_url_base: String,
    pub basic_info_url: String,
    pub real_time_url: String,
    pub exec_callback_url: String,
}

pub fn build_urls(
    http_server: &str,
    ws_server: &Option<String>,
    token: &str,
) -> Result<ConnectionUrls, ParseError> {
    fn get_port(url: &url::Url) -> String {
        if let Some(port) = url.port() {
            port.to_string()
        } else {
            if url.scheme() == "https" || url.scheme() == "wss" {
                "443".to_string()
            } else {
                "80".to_string()
            }
        }
    }

    let source_http_url = url::Url::parse(&http_server)?;

    let source_ws_url = if let Some(ws_server) = ws_server {
        url::Url::parse(&ws_server)?
    } else {
        let scheme = if source_http_url.scheme() == "https" {
            "wss"
        } else {
            "ws"
        };
        let host = source_http_url.host().ok_or(ParseError::EmptyHost)?;
        let port = get_port(&source_http_url);
        url::Url::parse(format!("{}://{}:{}", scheme, host, port).as_str())?
    };

    let http_url_base = {
        let scheme = source_http_url.scheme();
        let host = source_http_url.host().ok_or(ParseError::EmptyHost)?;
        let port = get_port(&source_http_url);
        format!("{}://{}:{}", scheme, host, port)
    };

    let ws_url_base = {
        let scheme = source_ws_url.scheme();
        let host = source_ws_url.host().ok_or(ParseError::EmptyHost)?;
        let port = get_port(&source_ws_url);
        format!("{}://{}:{}", scheme, host, port)
    };

    let basic_info_url = format!(
        "{}/api/clients/uploadBasicInfo?token={}",
        http_url_base, token
    );
    let real_time_url = format!("{}/api/clients/report?token={}", ws_url_base, token);
    let exec_callback_url = format!("{}/api/clients/task/result?token={}", http_url_base, token);

    let connection_urls = ConnectionUrls {
        http_url_base,
        ws_url_base,
        basic_info_url,
        real_time_url,
        exec_callback_url,
    };

    info!("URL 解析成功: {connection_urls:?}");

    Ok(connection_urls)
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
