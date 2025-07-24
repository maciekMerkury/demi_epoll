use std::{net::Ipv4Addr, time::Duration};
use super::raw;

pub fn duration_to_timespec(duration: Duration) -> raw::timespec {
    raw::timespec {
        tv_sec: duration.as_secs() as libc::time_t,
        tv_nsec: duration.subsec_nanos() as libc::c_long,
    }
}

pub fn ipv4_to_sockaddr_in(ip: Ipv4Addr, port: u16) -> raw::sockaddr_in {
    let ip_bytes = ip.octets();
    return raw::sockaddr_in {
        sin_family: raw::AF_INET as u16,
        sin_port: port.to_be(),
        sin_addr: raw::in_addr {
            s_addr: u32::from_be_bytes(ip_bytes),
        },
        sin_zero: [0; 8],
    };
}

pub fn sockaddr_in_to_ipv4(addr: raw::sockaddr_in) -> (Ipv4Addr, u16) {
    let ip_bytes = addr.sin_addr.s_addr.to_be_bytes();
    let ip = Ipv4Addr::from(ip_bytes);
    let port = u16::from_be(addr.sin_port);
    return (ip, port);
}

