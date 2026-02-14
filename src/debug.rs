use pnet::datalink;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::icmp::{IcmpPacket, IcmpTypes, MutableIcmpPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{Ipv4Packet, MutableIpv4Packet};
use pnet::packet::{MutablePacket, Packet};
use std::io::{Error, ErrorKind};

pub fn run_echo_server(interface_name: &str) -> std::io::Result<()> {
    let interfaces = datalink::interfaces();
    let interface = interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("Interface {} not found", interface_name),
            )
        })?;

    let mac = interface
        .mac
        .unwrap_or(pnet::util::MacAddr(0, 0, 0, 0, 0, 0));
    println!("MAC Address: {mac}");
    println!("Filtering for ICMP Echo Request and ARP packets...");

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
                let eth = EthernetPacket::new(packet).unwrap();
                let ethertype = eth.get_ethertype();

                if ethertype != EtherTypes::Ipv6 {
                    println!(
                        "DEBUG: Type: 0x{:04x}, Src: {}, Dst: {}",
                        ethertype.0,
                        eth.get_source(),
                        eth.get_destination()
                    );
                }

                match ethertype {
                    EtherTypes::Ipv4 => handle_ipv4(packet, &eth, &mut tx),
                    EtherTypes::Arp => handle_arp(packet, &eth, mac, &mut tx),
                    _ => {}
                }
            }
            Err(e) => return Err(e),
        }
    }
}

fn handle_ipv4(
    raw: &[u8],
    eth: &EthernetPacket,
    tx: &mut Box<dyn pnet::datalink::DataLinkSender>,
) {
    let Some(ip) = Ipv4Packet::new(eth.payload()) else {
        return;
    };
    let src = ip.get_source();
    let dst = ip.get_destination();

    if src.to_string() != "0.0.0.0" && dst.to_string() != "255.255.255.255" {
        println!("  -> IPv4: {src} -> {dst}, Proto: {:?}", ip.get_next_level_protocol());
    }

    if ip.get_next_level_protocol() != IpNextHeaderProtocols::Icmp {
        return;
    }

    let Some(icmp) = IcmpPacket::new(ip.payload()) else {
        return;
    };
    println!("    -> ICMP Type: {:?}", icmp.get_icmp_type());

    if icmp.get_icmp_type() != IcmpTypes::EchoRequest {
        return;
    }
    println!("ACTION: Echo Request detected! Replying...");

    let mut buf = vec![0u8; raw.len()];
    buf.copy_from_slice(raw);

    let mut new_eth = MutableEthernetPacket::new(&mut buf).unwrap();
    new_eth.set_source(eth.get_destination());
    new_eth.set_destination(eth.get_source());

    if let Some(mut new_ip) = MutableIpv4Packet::new(new_eth.payload_mut()) {
        new_ip.set_source(ip.get_destination());
        new_ip.set_destination(ip.get_source());

        if let Some(mut new_icmp) = MutableIcmpPacket::new(new_ip.payload_mut()) {
            new_icmp.set_icmp_type(IcmpTypes::EchoReply);
            new_icmp.set_checksum(pnet::packet::icmp::checksum(&new_icmp.to_immutable()));
        }
        new_ip.set_checksum(pnet::packet::ipv4::checksum(&new_ip.to_immutable()));
    }

    match tx.send_to(new_eth.packet(), None) {
        Some(Ok(_)) => println!("SUCCESS: ICMP Echo Reply sent!"),
        Some(Err(e)) => eprintln!("ERROR: Failed to send ICMP Reply: {e}"),
        None => {}
    }
}

fn handle_arp(
    _raw: &[u8],
    eth: &EthernetPacket,
    my_mac: pnet::util::MacAddr,
    tx: &mut Box<dyn pnet::datalink::DataLinkSender>,
) {
    let Some(arp) = ArpPacket::new(eth.payload()) else {
        return;
    };
    if arp.get_operation() != ArpOperations::Request {
        return;
    }
    println!("  -> ARP Request for: {}", arp.get_target_proto_addr());

    let mut buf = vec![0u8; 42];
    let mut new_eth = MutableEthernetPacket::new(&mut buf).unwrap();
    new_eth.set_destination(eth.get_source());
    new_eth.set_source(my_mac);
    new_eth.set_ethertype(EtherTypes::Arp);

    let mut new_arp = MutableArpPacket::new(new_eth.payload_mut()).unwrap();
    new_arp.set_hardware_type(ArpHardwareTypes::Ethernet);
    new_arp.set_protocol_type(EtherTypes::Ipv4);
    new_arp.set_hw_addr_len(6);
    new_arp.set_proto_addr_len(4);
    new_arp.set_operation(ArpOperations::Reply);
    new_arp.set_sender_hw_addr(my_mac);
    new_arp.set_sender_proto_addr(arp.get_target_proto_addr());
    new_arp.set_target_hw_addr(arp.get_sender_hw_addr());
    new_arp.set_target_proto_addr(arp.get_sender_proto_addr());

    match tx.send_to(new_eth.packet(), None) {
        Some(Ok(_)) => {
            println!(
                "        SUCCESS: ARP Reply sent for {}",
                arp.get_target_proto_addr()
            );
        }
        Some(Err(e)) => eprintln!("        ERROR: Failed to send ARP Reply: {e}"),
        None => {}
    }
}
