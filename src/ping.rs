use icmp_socket::packet::WithEchoRequest;
use icmp_socket::{
    IcmpSocket, IcmpSocket4, IcmpSocket6, Icmpv4Message, Icmpv4Packet, Icmpv6Message, Icmpv6Packet,
};
use miniserde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::time::Duration;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::net::TcpStream;
use tokio::net::lookup_host;
use tokio::time::Instant;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PingEvent {
    message: String,
    ping_task_id: u64,
    ping_type: String,
    ping_target: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PingEventCallback {
    #[serde(rename = "type")]
    pub type_str: String,
    pub task_id: u64,
    pub ping_type: String,
    pub value: Option<i64>,
    pub finished_at: String,
}

// 直接接收字符串参数，避免重复解析
pub async fn ping_target(utf8_str: &str) -> Result<PingEventCallback, String> {
    let ping_event: PingEvent =
        miniserde::json::from_str(utf8_str).map_err(|_| "无法解析 PingEvent".to_string())?;

    match ping_event.ping_type.as_str() {
        "icmp" => {
            #[cfg(not(target_os = "windows"))]
            if std::env::var("USER").unwrap_or_default() != "root" {
                return Err(String::from("无法在非特权环境下创建 Raw 套接字"));
            }

            match get_ip_from_string(&ping_event.ping_target).await {
                Ok(ip) => match ip {
                    IpAddr::V4(ip) => icmp_ipv4(ip, ping_event.ping_task_id).await,
                    IpAddr::V6(ip) => icmp_ipv6(ip, ping_event.ping_task_id).await,
                },
                Err(_) => Err(String::from("无法解析 IP 地址")),
            }
        }
        "tcp" => {
            let start_time = Instant::now();

            let ping = match tokio::time::timeout(
                Duration::from_secs(10),
                TcpStream::connect(&ping_event.ping_target), // 避免克隆
            )
            .await
            {
                Err(_) => Err("Tcping 超时".to_string()),
                Ok(Ok(_)) => Ok(()),
                Ok(Err(_)) => Err("无法连接".to_string()),
            };

            let rtt = start_time.elapsed();

            let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
            let finished_at = now.format(&Rfc3339).unwrap_or_default();

            if let Ok(()) = ping {
                Ok(PingEventCallback {
                    type_str: String::from("ping_result"),
                    task_id: ping_event.ping_task_id,
                    ping_type: String::from("tcp"),
                    value: i64::try_from(rtt.as_millis()).ok(),
                    finished_at,
                })
            } else {
                Ok(PingEventCallback {
                    type_str: String::from("ping_result"),
                    task_id: ping_event.ping_task_id,
                    ping_type: String::from("tcp"),
                    value: None,
                    finished_at,
                })
            }
        }
        "http" => {
            let start_time = Instant::now();
            let result = ureq::get(&ping_event.ping_target) // 避免克隆
                .header("User-Agent", "curl/11.45.14")
                .call()
                .is_ok();

            let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
            let finished_at = now.format(&Rfc3339).unwrap_or_default();

            if result {
                Ok(PingEventCallback {
                    type_str: String::from("ping_result"),
                    task_id: ping_event.ping_task_id,
                    ping_type: String::from("http"),
                    value: i64::try_from(start_time.elapsed().as_millis()).ok(),
                    finished_at,
                })
            } else {
                Ok(PingEventCallback {
                    type_str: String::from("ping_result"),
                    task_id: ping_event.ping_task_id,
                    ping_type: String::from("http"),
                    value: None,
                    finished_at,
                })
            }
        }
        _ => Err(format!("Ping Error: Not Support: {}", ping_event.ping_type)),
    }
}

pub async fn get_ip_from_string(host_or_ip: &str) -> Result<IpAddr, String> {
    if let Ok(ip) = IpAddr::from_str(host_or_ip) {
        return Ok(ip);
    }

    let host_with_port = format!("{}:80", host_or_ip);
    match lookup_host(&host_with_port).await {
        // 避免克隆
        Ok(mut ip_addresses) => {
            if let Some(first_socket_addr) = ip_addresses.next() {
                Ok(first_socket_addr.ip())
            } else {
                Err(format!(
                    "No IP addresses found for the domain: {}",
                    host_or_ip
                ))
            }
        }
        Err(e) => Err(format!("Error looking up domain: {}", e)),
    }
}

pub async fn icmp_ipv4(ip: Ipv4Addr, task_id: u64) -> Result<PingEventCallback, String> {
    let Ok(mut socket4) = IcmpSocket4::new() else {
        return Err(String::from("无法创建 Raw 套接字"));
    };

    if socket4
        .bind("0.0.0.0".parse::<Ipv4Addr>().unwrap())
        .is_err()
    {
        return Err(String::from("无法绑定 Raw 套接字"));
    }

    let packet = Icmpv4Packet::with_echo_request(
        42,
        0,
        vec![
            0x20, 0x20, 0x75, 0x73, 0x74, 0x20, 0x61, 0x20, 0x66, 0x6c, 0x65, 0x73, 0x68, 0x20,
            0x77, 0x6f, 0x75, 0x6e, 0x64, 0x20, 0x20, 0x74, 0x69, 0x73, 0x20, 0x62, 0x75, 0x74,
            0x20, 0x61, 0x20, 0x73, 0x63, 0x72, 0x61, 0x74, 0x63, 0x68, 0x20, 0x20, 0x6b, 0x6e,
            0x69, 0x67, 0x68, 0x74, 0x73, 0x20, 0x6f, 0x66, 0x20, 0x6e, 0x69, 0x20, 0x20, 0x20,
        ],
    )
    .unwrap();

    let send_time = Instant::now();
    if socket4.send_to(ip, packet).is_err() {
        let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        let finished_at = now.format(&Rfc3339).unwrap_or_default();

        return Ok(PingEventCallback {
            type_str: String::from("ping_result"),
            task_id,
            ping_type: String::from("icmp"),
            value: None,
            finished_at,
        });
    }

    socket4.set_timeout(Some(Duration::from_secs(3)));

    let Ok((resp, _)) = socket4.rcv_from() else {
        let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        let finished_at = now.format(&Rfc3339).unwrap_or_default();

        return Ok(PingEventCallback {
            type_str: String::from("ping_result"),
            task_id,
            ping_type: String::from("icmp"),
            value: None,
            finished_at,
        });
    };

    let rtt = send_time.elapsed();

    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let finished_at = now.format(&Rfc3339).unwrap_or_default();

    if let Icmpv4Message::EchoReply {
        identifier: _,
        sequence: _sequence,
        payload: _payload,
    } = resp.message
    {
        Ok(PingEventCallback {
            type_str: String::from("ping_result"),
            task_id,
            ping_type: String::from("icmp"),
            value: i64::try_from(rtt.as_millis()).ok(),
            finished_at,
        })
    } else {
        Ok(PingEventCallback {
            type_str: String::from("ping_result"),
            task_id,
            ping_type: String::from("icmp"),
            value: None,
            finished_at,
        })
    }
}

pub async fn icmp_ipv6(ip: Ipv6Addr, task_id: u64) -> Result<PingEventCallback, String> {
    let Ok(mut socket6) = IcmpSocket6::new() else {
        return Err(String::from("无法创建 Raw 套接字"));
    };

    if socket6.bind("::0".parse::<Ipv6Addr>().unwrap()).is_err() {
        return Err(String::from("无法绑定 Raw 套接字"));
    }

    let packet = Icmpv6Packet::with_echo_request(
        42,
        0,
        vec![
            0x20, 0x20, 0x75, 0x73, 0x74, 0x20, 0x61, 0x20, 0x66, 0x6c, 0x65, 0x73, 0x68, 0x20,
            0x77, 0x6f, 0x75, 0x6e, 0x64, 0x20, 0x20, 0x74, 0x69, 0x73, 0x20, 0x62, 0x75, 0x74,
            0x20, 0x61, 0x20, 0x73, 0x63, 0x72, 0x61, 0x74, 0x63, 0x68, 0x20, 0x20, 0x6b, 0x6e,
            0x69, 0x67, 0x68, 0x74, 0x73, 0x20, 0x6f, 0x66, 0x20, 0x6e, 0x69, 0x20, 0x20, 0x20,
        ],
    )
    .unwrap();

    let send_time = Instant::now();

    if socket6.send_to(ip, packet).is_err() {
        let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        let finished_at = now.format(&Rfc3339).unwrap_or_default();

        return Ok(PingEventCallback {
            type_str: String::from("ping_result"),
            task_id,
            ping_type: String::from("icmp"),
            value: None,
            finished_at,
        });
    }

    socket6.set_timeout(Some(Duration::from_secs(3)));

    let Ok((resp, _)) = socket6.rcv_from() else {
        let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
        let finished_at = now.format(&Rfc3339).unwrap_or_default();

        return Ok(PingEventCallback {
            type_str: String::from("ping_result"),
            task_id,
            ping_type: String::from("icmp"),
            value: None,
            finished_at,
        });
    };

    let rtt = send_time.elapsed();

    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let finished_at = now.format(&Rfc3339).unwrap_or_default();

    if let Icmpv6Message::EchoReply {
        identifier: _,
        sequence: _sequence,
        payload: _payload,
    } = resp.message
    {
        Ok(PingEventCallback {
            type_str: String::from("ping_result"),
            task_id,
            ping_type: String::from("icmp"),
            value: i64::try_from(rtt.as_millis()).ok(),
            finished_at,
        })
    } else {
        Ok(PingEventCallback {
            type_str: String::from("ping_result"),
            task_id,
            ping_type: String::from("icmp"),
            value: None,
            finished_at,
        })
    }
}
