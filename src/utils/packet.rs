use etherparse::{InternetSlice, SlicedPacket, TransportSlice};

#[derive(Debug)]
pub struct PacketInfo {
    pub id: usize,
    pub timestamp: String,
    pub src_ip: String,
    pub dst_ip: String,
    pub protocol: String,
    pub length: usize,
    pub info: String,
}

pub fn parse_packet(id: usize, timestamp: String, data: &[u8]) -> PacketInfo {
    match SlicedPacket::from_ethernet(data) {
        Ok(packet_info) => {
            let mut src_addr = "".to_string();
            let mut dst_addr = "".to_string();
            let mut protocol = "Unknown".to_string();
            let mut info = String::new();

            if let Some(ip_slice) = packet_info.net {
                match ip_slice {
                    InternetSlice::Ipv4(ipv4) => {
                        src_addr =
                            crate::utils::pretty_print::pretty_print_ipv4(&ipv4.header().source());
                        dst_addr = crate::utils::pretty_print::pretty_print_ipv4(
                            &ipv4.header().destination(),
                        );
                        protocol = format!("IPv4/{:?}", ipv4.header().protocol());
                    }
                    InternetSlice::Ipv6(ipv6) => {
                        src_addr =
                            crate::utils::pretty_print::pretty_print_ipv6(&ipv6.header().source());
                        dst_addr = crate::utils::pretty_print::pretty_print_ipv6(
                            &ipv6.header().destination(),
                        );
                        protocol = format!("IPv6/{:?}", ipv6.header().next_header());
                    }
                    InternetSlice::Arp(arp) => {
                        if arp.sender_hw_addr().len() == 6 {
                            src_addr = crate::utils::pretty_print::pretty_print_mac(
                                arp.sender_hw_addr()[..6].try_into().unwrap(),
                            );
                            dst_addr = crate::utils::pretty_print::pretty_print_mac(
                                arp.target_hw_addr()[..6].try_into().unwrap(),
                            );
                        }
                        protocol = "ARP".to_string();
                    }
                }
            }

            if let Some(transport_slice) = packet_info.transport {
                match transport_slice {
                    TransportSlice::Tcp(tcp) => {
                        protocol = "TCP".to_string();
                        info = format!(
                            "{}:{} -> {}:{}",
                            src_addr,
                            tcp.source_port(),
                            dst_addr,
                            tcp.destination_port()
                        );
                    }
                    TransportSlice::Udp(udp) => {
                        protocol = "UDP".to_string();
                        info = format!(
                            "{}:{} -> {}:{}",
                            src_addr,
                            udp.source_port(),
                            dst_addr,
                            udp.destination_port()
                        );
                    }
                    TransportSlice::Icmpv4(_) => {
                        protocol = "ICMPv4".to_string();
                        info = format!("{} -> {}", src_addr, dst_addr);
                    }
                    TransportSlice::Icmpv6(_) => {
                        protocol = "ICMPv6".to_string();
                        info = format!("{} -> {}", src_addr, dst_addr);
                    }
                }
            } else if !src_addr.is_empty() && !dst_addr.is_empty() {
                info = format!("{} -> {}", src_addr, dst_addr);
            }

            PacketInfo {
                id,
                timestamp,
                src_ip: src_addr,
                dst_ip: dst_addr,
                protocol,
                length: data.len(),
                info,
            }
        }
        Err(_) => PacketInfo {
            id,
            timestamp,
            src_ip: "N/A".to_string(),
            dst_ip: "N/A".to_string(),
            protocol: "Raw".to_string(),
            length: data.len(),
            info: format!("Raw packet ({} bytes)", data.len()),
        },
    }
}
