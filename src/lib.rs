// Flight - XDP/AF_XDP based kernel-bypass networking library

pub mod debug {
    use pnet::datalink::{self};
    use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
    use pnet::packet::ipv4::{Ipv4Packet, MutableIpv4Packet};
    use pnet::packet::icmp::{IcmpPacket, MutableIcmpPacket, IcmpTypes};
    use pnet::packet::arp::{ArpPacket, MutableArpPacket, ArpOperations, ArpHardwareTypes};
    use pnet::packet::ip::IpNextHeaderProtocols;
    use pnet::packet::{Packet, MutablePacket};
    use std::io::{Error, ErrorKind};

    pub fn run_echo_server(interface_name: &str) -> std::io::Result<()> {
        // Find the network interface
        let interfaces = datalink::interfaces();
        let interface = interfaces
            .into_iter()
            .find(|iface| iface.name == interface_name)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("Interface {} not found", interface_name)))?;

        println!("MAC Address: {}", interface.mac.unwrap_or(pnet::util::MacAddr(0,0,0,0,0,0)));
        println!("Filtering for ICMP Echo Request and ARP packets...");

        // Create a channel to receive and send packets with promiscuous mode enabled
        let config = datalink::Config {
            promiscuous: true,
            ..Default::default()
        };

        let (mut tx, mut rx) = match datalink::channel(&interface, config) {
            Ok(datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
            Ok(_) => return Err(Error::new(ErrorKind::Other, "Unhandled channel type")),
            Err(e) => return Err(e),
        };

        loop {
            match rx.next() {
                Ok(packet) => {
                    let ethernet_packet = EthernetPacket::new(packet).unwrap();
                    let ethertype = ethernet_packet.get_ethertype();

                    // Log every packet EXCEPT IPv6 to reduce noise
                    if ethertype != EtherTypes::Ipv6 {
                        println!("DEBUG: Packet received. Type: 0x{:04x}, Src: {}, Dst: {}", 
                            ethertype.0, ethernet_packet.get_source(), ethernet_packet.get_destination());
                    }

                    if ethertype == EtherTypes::Ipv4 {
                        if let Some(ipv4_packet) = Ipv4Packet::new(ethernet_packet.payload()) {
                            // Log all IPv4 packets
                            let src = ipv4_packet.get_source();
                            let dst = ipv4_packet.get_destination();
                            let proto = ipv4_packet.get_next_level_protocol();

                            if src.to_string() != "0.0.0.0" && dst.to_string() != "255.255.255.255" {
                                println!("  -> IPv4: {} -> {}, Proto: {:?}", src, dst, proto);
                            }

                            if proto == IpNextHeaderProtocols::Icmp {
                                if let Some(icmp_packet) = IcmpPacket::new(ipv4_packet.payload()) {
                                    println!("    -> ICMP Type: {:?}", icmp_packet.get_icmp_type());
                                    
                                    if icmp_packet.get_icmp_type() == IcmpTypes::EchoRequest {
                                        println!(
                                            "ACTION: Echo Request detected! Replying..."
                                        );

                                        // Construct the response packet
                                        let mut response_buffer = vec![0u8; packet.len()];
                                        response_buffer.copy_from_slice(packet);

                                        let mut new_eth_packet = MutableEthernetPacket::new(&mut response_buffer).unwrap();

                                        // Swap MAC addresses
                                        new_eth_packet.set_source(ethernet_packet.get_destination());
                                        new_eth_packet.set_destination(ethernet_packet.get_source());

                                        // Swap IP addresses and update ICMP
                                        if let Some(mut new_ipv4_packet) = MutableIpv4Packet::new(new_eth_packet.payload_mut()) {
                                            new_ipv4_packet.set_source(ipv4_packet.get_destination());
                                            new_ipv4_packet.set_destination(ipv4_packet.get_source());

                                            // Convert ICMP Echo Request to Echo Reply (Type 0)
                                            if let Some(mut new_icmp_packet) = MutableIcmpPacket::new(new_ipv4_packet.payload_mut()) {
                                                new_icmp_packet.set_icmp_type(IcmpTypes::EchoReply);

                                                // Recalculate ICMP checksum
                                                let checksum = pnet::packet::icmp::checksum(&new_icmp_packet.to_immutable());
                                                new_icmp_packet.set_checksum(checksum);
                                            }

                                            // Recalculate IPv4 checksum
                                            let checksum = pnet::packet::ipv4::checksum(&new_ipv4_packet.to_immutable());
                                            new_ipv4_packet.set_checksum(checksum);
                                        }

                                        // Send the packet
                                        if let Some(res) = tx.send_to(new_eth_packet.packet(), None) {
                                            if let Err(e) = res {
                                                 eprintln!("ERROR: Failed to send ICMP Reply: {}", e);
                                            } else {
                                                 println!("SUCCESS: ICMP Echo Reply sent!");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if ethertype == EtherTypes::Arp {
                        if let Some(arp_packet) = ArpPacket::new(ethernet_packet.payload()) {
                             if arp_packet.get_operation() == ArpOperations::Request {
                                println!(
                                    "  -> ARP Request for: {}",
                                    arp_packet.get_target_proto_addr()
                                );

                                let mut response_buffer = vec![0u8; 42]; // Min size for Eth + ARP
                                let mut new_eth_packet = MutableEthernetPacket::new(&mut response_buffer).unwrap();

                                new_eth_packet.set_destination(ethernet_packet.get_source());
                                new_eth_packet.set_source(interface.mac.unwrap_or(pnet::util::MacAddr(0,0,0,0,0,0))); 
                                new_eth_packet.set_ethertype(EtherTypes::Arp);

                                let mut new_arp_packet = MutableArpPacket::new(new_eth_packet.payload_mut()).unwrap();
                                new_arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
                                new_arp_packet.set_protocol_type(EtherTypes::Ipv4);
                                new_arp_packet.set_hw_addr_len(6);
                                new_arp_packet.set_proto_addr_len(4);
                                new_arp_packet.set_operation(ArpOperations::Reply);
                                
                                new_arp_packet.set_sender_hw_addr(interface.mac.unwrap_or(pnet::util::MacAddr(0,0,0,0,0,0)));
                                new_arp_packet.set_sender_proto_addr(arp_packet.get_target_proto_addr());
                                
                                new_arp_packet.set_target_hw_addr(arp_packet.get_sender_hw_addr());
                                new_arp_packet.set_target_proto_addr(arp_packet.get_sender_proto_addr());

                                if let Some(res) = tx.send_to(new_eth_packet.packet(), None) {
                                    if let Err(e) = res {
                                        eprintln!("        ERROR: Failed to send ARP Reply: {}", e);
                                    } else {
                                        println!("        SUCCESS: ARP Reply sent for {}", arp_packet.get_target_proto_addr());
                                    }
                                }
                             }
                        }
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
}
