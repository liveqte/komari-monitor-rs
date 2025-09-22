use log::Level;
use url::ParseError;
use crate::command_parser::LogLevel;

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

pub fn build_urls(http_server: String, ws_server: Option<String>, token: String) -> Result<(String, String, String), ParseError> {
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

        let port = if let Some(port) = source_http_url.port() {
            port.to_string()
        } else {
            if source_http_url.scheme() == "https" {
                "443".to_string()
            } else {
                "80".to_string()
            }
        };

        url::Url::parse(format!("{}://{}:{}", scheme, host, port).as_str())?
    };
    
    let http_server = {
        let scheme = source_http_url.scheme();
        let host = source_http_url.host().ok_or(ParseError::EmptyHost)?;
        let port = if let Some(port) = source_http_url.port() {
            port.to_string()
        } else {
            if source_http_url.scheme() == "https" {
                "443".to_string()
            } else {
                "80".to_string()
            }
        };
        
        format!("{}://{}:{}", scheme, host, port)
    }

    let basic_info_url = format!(
        "{}/api/clients/uploadBasicInfo?token={}",
        http_server, token
    );
    let real_time_url = format!("{}/api/clients/report?token={}", args.ws_server, args.token);
    let exec_callback_url = format!(
        "{}/api/clients/task/result?token={}",
        http_server, token
    );
}