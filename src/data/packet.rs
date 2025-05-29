use std::net::IpAddr;

use etherparse::{InternetSlice, SlicedPacket, TransportSlice};

#[derive(Debug)]
pub struct PacketInfo {
    pub id: usize,
    pub timestamp: String,
    pub src_ip: Option<IpAddr>,
    pub src_port: Option<u16>,
    pub dst_ip: Option<IpAddr>,
    pub dst_port: Option<u16>,
    pub protocol: String,
    pub length: usize,
}

pub fn parse_packet(id: usize, timestamp: String, data: &[u8]) -> PacketInfo {
    match SlicedPacket::from_ethernet(data) {
        Ok(packet_info) => {
            let mut src_ip: Option<IpAddr> = None;
            let mut dst_ip: Option<IpAddr> = None;
            let mut src_port: Option<u16> = None;
            let mut dst_port: Option<u16> = None;
            let mut protocol = "Unknown".to_string();

            if let Some(ip_slice) = packet_info.net {
                match ip_slice {
                    InternetSlice::Ipv4(ipv4) => {
                        src_ip = Some(IpAddr::V4(ipv4.header().source().into()));
                        dst_ip = Some(IpAddr::V4(ipv4.header().destination().into()));
                        protocol = format!("IPv4/{:?}", ipv4.header().protocol());
                    }
                    InternetSlice::Ipv6(ipv6) => {
                        src_ip = Some(IpAddr::V6(ipv6.header().source().into()));
                        dst_ip = Some(IpAddr::V6(ipv6.header().destination().into()));
                        protocol = format!("IPv6/{:?}", ipv6.header().next_header());
                    }
                    InternetSlice::Arp(_) => {
                        protocol = "ARP".to_string();
                    }
                }
            }

            if let Some(transport_slice) = packet_info.transport {
                match transport_slice {
                    TransportSlice::Tcp(tcp) => {
                        src_port = Some(tcp.source_port());
                        dst_port = Some(tcp.destination_port());
                        protocol = "TCP".to_string();
                    }
                    TransportSlice::Udp(udp) => {
                        src_port = Some(udp.source_port());
                        dst_port = Some(udp.destination_port());
                        protocol = "UDP".to_string();
                    }
                    TransportSlice::Icmpv4(_) => {
                        protocol = "ICMPv4".to_string();
                    }
                    TransportSlice::Icmpv6(_) => {
                        protocol = "ICMPv6".to_string();
                    }
                }
            }

            PacketInfo {
                id,
                timestamp,
                src_ip,
                src_port,
                dst_ip,
                dst_port,
                protocol,
                length: data.len(),
            }
        }
        Err(_) => PacketInfo {
            id,
            timestamp,
            src_ip: None,
            src_port: None,
            dst_ip: None,
            dst_port: None,
            protocol: "Unknown".to_string(),
            length: data.len(),
        },
    }
}
