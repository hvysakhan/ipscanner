// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pnet::datalink::{self, NetworkInterface};
use pnet::ipnetwork::{IpNetwork, Ipv4Network};
// use tokio::sync::mpsc;
// use std::sync::{Arc};
// use futures::lock::Mutex as FMutex;
use network_interface::{NetworkInterface as NetworkInterfaceNW, NetworkInterfaceConfig};
use serde_json::{json, Value};

use std::env;
// use std::io::{self, Write};
use std::net::{ IpAddr, Ipv4Addr};
// use std::process;
use tokio::task::JoinHandle;

use pnet::datalink::{Channel, MacAddr};

use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::EtherTypes;
use pnet::packet::ethernet::MutableEthernetPacket;
use pnet::packet::{MutablePacket, Packet};



// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn list_network_interfaces() -> Vec<Value> {
    let network_interfaces = NetworkInterfaceNW::show().unwrap();
    let mut itf1: Vec<Value> = vec![];
    for (i,itf) in network_interfaces.iter().enumerate() {
        // println!("{:?}", itf.name);
        itf1.push(json!({
            "id": i,
            "name": itf.name
        }));
    }
    return itf1;
}


fn is_private_ipv4_address(ip: std::net::Ipv4Addr) -> bool {
    let octets = ip.octets();
    match octets[0] {
        10 => true,
        172 => octets[1] >= 16 && octets[1] <= 31,
        192 => octets[1] == 168,
        _ => false,
    }
}

async fn is_pingable(ip: std::net::IpAddr) -> bool {
    // let timeout = Duration::from_millis(50000);
    let payload = [0; 8];
    match surge_ping::ping(ip, &payload).await {
        Ok((_, _)) => true,
        Err(_) => false,
    }
}


#[tauri::command]
async fn list_ips(interface_name: String)-> Vec<Value> {

    let interface_names_match =
        |iface: &NetworkInterface| iface.name == interface_name;

    // Find the network interface with the provided name
    let interfaces = datalink::interfaces();
    // println!("{:?}", interfaces);


    let interface = interfaces.into_iter()
                               .filter(interface_names_match)
                               .next()
                               .unwrap();
    println!("{:?}", interface.ips);

    // let interfaces = datalink::interfaces();
    let mut pingable_ips_json: Vec<Value> = vec![];
    let ips = interface.ips.clone();
    for ip in ips {
        if let IpNetwork::V4(net) = ip {
            if net.ip().is_loopback() || net.ip().is_link_local() {
                continue;
            }
            if is_private_ipv4_address(net.ip()) {
                println!("Scanning network: {}", net);
                let network = Ipv4Network::new(net.ip(), net.prefix()).unwrap();

                // let mut pingable_ips: Vec<std::net::Ipv4Addr> = Vec::new();

                let mut handles = Vec::new();
                for ip in network.iter() {
                    let handle: JoinHandle<Option<Ipv4Addr>> = tokio::spawn(async move {
                        if is_pingable(std::net::IpAddr::V4(ip)).await {
                            // println!("{}: online", ip);
                            return Some(ip);
                        } else {
                            return None;
                            // println!("{}: offline", ip);
                        }
                    });

                    handles.push(handle);
                }
                println!("done");
                let mut ips = Vec::new();
                for handle in handles {


                    match handle.await.unwrap() {
                        Some(ip) => ips.push(ip),
                        None => (),
                    }
                    // ips.push(handle.await.unwrap());
                    // ips.push(handle.await.unwrap());
                }
                for (i, ip) in ips.iter().enumerate() {
                    pingable_ips_json.push(
                        json!({
                            "id": i,
                            "name": ip.to_string()
                        })
                    );
                    println!("ip: {:?}", ip);
                }


                // let pingable_ips = Arc::new(FMutex::new(Vec::new()));
                
                // let (tx, mut rx) = mpsc::channel(256);

                // for ip in network.iter() {
                //     let tx = tx.clone();
                //     // let pingable_ips = pingable_ips.clone();
                //     // let ip_clone = *ip;

                //     tokio::spawn(async move {
                //         if is_pingable(std::net::IpAddr::V4(ip)).await {
                //             tx.send(ip).await.unwrap();
                //         } else {
                //             // tx.send(None).await.unwrap();
                //         }
                //     });

                // }

                // drop(tx);
                // println!("Waiting for results...");


                // while let Some(result) = rx.recv().await {
                //     println!("Received: {:?}", result);
                //     // println!("some ip:{:?}", Some(ip));
                    
                //     pingable_ips.lock().await.push(result);
                    
                    
                // }
                // println!("Done!");

                // println!("Pingable IPs: {:?}", *pingable_ips.lock().await);

                // for (i,pingable_ip) in pingable_ips.lock().await.iter().enumerate() {
                //     println!("{}: online", pingable_ip);
                //     pingable_ips_json.push(
                //         json!({
                //             "id": i,
                //             "name": pingable_ip.to_string()
                //         })
                //     );
                // }
            }
        }
    }
    return pingable_ips_json;

}

#[tauri::command]
fn get_mac_through_arp(interface: String, target_ip: String) -> String {

    let interface_names_match =
        |iface: &NetworkInterface| iface.name == interface;

    let interfaces = datalink::interfaces();
    // println!("{:?}", interfaces);


    let interface = interfaces.into_iter()
                               .filter(interface_names_match)
                               .next()
                               .unwrap();

    let target_ip = target_ip.parse::<Ipv4Addr>().unwrap();

    let source_ip = interface
        .ips
        .iter()
        .find(|ip| ip.is_ipv4())
        .map(|ip| match ip.ip() {
            IpAddr::V4(ip) => ip,
            _ => unreachable!(),
        })
        .unwrap();
    if(source_ip == target_ip){
        return interface.mac.unwrap().to_string();
    }
    let (mut sender, mut receiver) = match pnet::datalink::channel(&interface, Default::default()) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("Unknown channel type"),
        Err(e) => panic!("Error happened {}", e),
    };

    let mut ethernet_buffer = [0u8; 42];
    let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

    ethernet_packet.set_destination(MacAddr::broadcast());
    ethernet_packet.set_source(interface.mac.unwrap());
    ethernet_packet.set_ethertype(EtherTypes::Arp);

    let mut arp_buffer = [0u8; 28];
    let mut arp_packet = MutableArpPacket::new(&mut arp_buffer).unwrap();

    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Request);
    arp_packet.set_sender_hw_addr(interface.mac.unwrap());
    arp_packet.set_sender_proto_addr(source_ip);
    arp_packet.set_target_hw_addr(MacAddr::zero());
    arp_packet.set_target_proto_addr(target_ip);

    ethernet_packet.set_payload(arp_packet.packet_mut());

    sender
        .send_to(ethernet_packet.packet(), None)
        .unwrap()
        .unwrap();

    println!("Sent ARP request");

    loop {
        let buf = receiver.next().unwrap();
        let arp = ArpPacket::new(&buf[MutableEthernetPacket::minimum_packet_size()..]).unwrap();
        if arp.get_sender_proto_addr() == target_ip
            && arp.get_target_hw_addr() == interface.mac.unwrap()
        {
            println!("Received reply");
            return arp.get_sender_hw_addr().to_string();
        }
        else{
            return "Not Found".to_string();
        }
    }

    // while let buf = receiver.next().unwrap() {
    //     let arp = ArpPacket::new(&buf[MutableEthernetPacket::minimum_packet_size()..]).unwrap();
    //     if arp.get_sender_proto_addr() == target_ip
    //         && arp.get_target_hw_addr() == interface.mac.unwrap()
    //     {
    //         println!("Received reply");
    //         return arp.get_sender_hw_addr().to_string();
    //     }
    //     else{
    //         return "Not Found".to_string();
    //     }
    // }
    // panic!("Never reach here");
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_network_interfaces, list_ips, get_mac_through_arp])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
