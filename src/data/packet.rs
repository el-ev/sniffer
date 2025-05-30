use std::{net::IpAddr, sync::Arc};

use etherparse::{InternetSlice, SlicedPacket, TransportSlice};

#[derive(Debug, Clone)]
pub struct PacketInfo {
    pub id: usize,
    pub timestamp: String,
    pub src_addr: Option<Result<IpAddr, String>>,
    pub src_port: Option<u16>,
    pub dst_addr: Option<Result<IpAddr, String>>,
    pub dst_port: Option<u16>,
    pub protocol: String,
    pub length: usize,
    pub data: Arc<[u8]>,
}

pub fn parse_packet(id: usize, timestamp: String, data: Arc<[u8]>) -> PacketInfo {
    let mut src_addr: Option<Result<IpAddr, String>> = None;
    let mut dst_addr: Option<Result<IpAddr, String>> = None;
    let mut src_port: Option<u16> = None;
    let mut dst_port: Option<u16> = None;
    let mut protocol = "Unknown".to_string();
    match SlicedPacket::from_ethernet(&data) {
        Ok(packet_info) => {
            if let Some(ip_slice) = packet_info.net {
                match ip_slice {
                    InternetSlice::Ipv4(ipv4) => {
                        src_addr = Some(Ok(IpAddr::V4(ipv4.header().source().into())));
                        dst_addr = Some(Ok(IpAddr::V4(ipv4.header().destination().into())));
                        protocol = format!("IPv4/{:?}", ipv4.header().protocol());
                    }
                    InternetSlice::Ipv6(ipv6) => {
                        src_addr = Some(Ok(IpAddr::V6(ipv6.header().source().into())));
                        dst_addr = Some(Ok(IpAddr::V6(ipv6.header().destination().into())));
                        protocol = format!("IPv6/{:?}", ipv6.header().next_header());
                    }
                    InternetSlice::Arp(arp) => {
                        src_addr = Some(Err(format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", 
                            arp.sender_hw_addr()[0], arp.sender_hw_addr()[1], arp.sender_hw_addr()[2],
                            arp.sender_hw_addr()[3], arp.sender_hw_addr()[4], arp.sender_hw_addr()[5])));
                        dst_addr = Some(Err(format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", 
                            arp.target_hw_addr()[0], arp.target_hw_addr()[1], arp.target_hw_addr()[2],
                            arp.target_hw_addr()[3], arp.target_hw_addr()[4], arp.target_hw_addr()[5])));
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
        }
        Err(_) => {
            protocol = "Unknown".to_string();
        }
    }
    PacketInfo {
        id,
        timestamp,
        src_addr,
        src_port,
        dst_addr,
        dst_port,
        protocol,
        length: data.len(),
        data,
    }
}
