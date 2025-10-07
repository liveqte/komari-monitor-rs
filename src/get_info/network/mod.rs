use crate::data_struct::{Connections, Network};
use log::trace;
use sysinfo::Networks;
#[cfg(target_os = "linux")]
mod netlink;

pub static mut DURATION: f64 = 0.0;
pub fn realtime_network(network: &Networks) -> Network {
    let mut total_up = 0;
    let mut total_down = 0;
    let mut up = 0;
    let mut down = 0;

    for (name, data) in network {
        if name.contains("lo")
            || name.contains("veth")
            || name.contains("docker")
            || name.contains("tun")
            || name.contains("br")
            || name.contains("tap")
        {
            continue;
        }
        total_up += data.total_transmitted();
        total_down += data.total_received();
        up += data.transmitted();
        down += data.received();
    }

    unsafe {
        let network_info = Network {
            up: (up as f64 / (DURATION / 1000.0)) as u64,
            down: (down as f64 / (DURATION / 1000.0)) as u64,
            total_up,
            total_down,
        };
        trace!("REALTIME NETWORK 获取成功: {network_info:?}");
        network_info
    }
}

#[cfg(target_os = "linux")]
pub fn realtime_connections() -> Connections {
    use netlink::connections_count_with_protocol;
    let tcp4 =
        connections_count_with_protocol(libc::AF_INET as u8, libc::IPPROTO_TCP as u8).unwrap_or(0);
    let tcp6 =
        connections_count_with_protocol(libc::AF_INET6 as u8, libc::IPPROTO_TCP as u8).unwrap_or(0);
    let udp4 =
        connections_count_with_protocol(libc::AF_INET as u8, libc::IPPROTO_UDP as u8).unwrap_or(0);
    let udp6 =
        connections_count_with_protocol(libc::AF_INET6 as u8, libc::IPPROTO_UDP as u8).unwrap_or(0);
    let connections = Connections {
        tcp: tcp4 + tcp6,
        udp: udp4 + udp6,
    };
    trace!("REALTIME CONNECTIONS 获取成功: {:?}", connections);
    connections
}

#[cfg(target_os = "windows")]
pub fn realtime_connections() -> Connections {
    use netstat2::{ProtocolFlags, ProtocolSocketInfo, iterate_sockets_info_without_pids};
    let proto_flags = ProtocolFlags::TCP | ProtocolFlags::UDP;

    let Ok(sockets_iterator) = iterate_sockets_info_without_pids(proto_flags) else {
        let connections = Connections { tcp: 0, udp: 0 };
        trace!("REALTIME CONNECTIONS 获取成功: {connections:?}");
        return connections;
    };

    let (mut tcp_count, mut udp_count) = (0, 0);

    for info_result in sockets_iterator.flatten() {
        match info_result.protocol_socket_info {
            ProtocolSocketInfo::Tcp(_) => tcp_count += 1,
            ProtocolSocketInfo::Udp(_) => udp_count += 1,
        }
    }

    let connections = Connections {
        tcp: tcp_count,
        udp: udp_count,
    };
    trace!("REALTIME CONNECTIONS 获取成功: {connections:?}");
    connections
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn realtime_connections() -> Connections {
    let connections = Connections { tcp: 0, udp: 0 };
    trace!("REALTIME CONNECTIONS 获取成功: {:?}", connections);
    connections
}
