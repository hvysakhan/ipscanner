// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pnet::datalink::{self, NetworkInterface};
use pnet::ipnetwork::{IpNetwork, Ipv4Network};
use tokio::sync::mpsc;
use std::sync::{Arc};
use futures::lock::Mutex as FMutex;
use network_interface::{NetworkInterface as NetworkInterfaceNW, NetworkInterfaceConfig};
use serde_json::{json, Value};

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

                // for ip in network.iter() {
                //     // println!("{}", ip);
                //     if is_pingable(std::net::IpAddr::V4(ip)).await{
                //         println!("{}:online", ip);
                //     }else{
                //         println!("{}: offline", ip);

                //     }
                // }

                // let mut handles = Vec::new();
                // for ip in network.iter() {
                //     let handle: JoinHandle<()> = tokio::spawn(async move {
                //         if is_pingable(std::net::IpAddr::V4(ip)).await {
                //             println!("{}: online", ip);
                //         } else {
                //             // println!("{}: offline", ip);
                //         }
                //     });

                //     handles.push(handle);
                // }

                // for handle in handles {
                //     handle.await.unwrap();
                // }

                let pingable_ips = Arc::new(FMutex::new(Vec::new()));
                
                let (tx, mut rx) = mpsc::channel(256);

                for ip in network.iter() {
                    let tx = tx.clone();
                    // let pingable_ips = pingable_ips.clone();
                    // let ip_clone = *ip;

                    tokio::spawn(async move {
                        if is_pingable(std::net::IpAddr::V4(ip)).await {
                            tx.send(ip).await.unwrap();
                        } else {
                            // tx.send(None).await.unwrap();
                        }
                    });

                }

                drop(tx);
                println!("Waiting for results...");


                while let Some(result) = rx.recv().await {
                    println!("Received: {:?}", result);
                    // println!("some ip:{:?}", Some(ip));
                    
                    pingable_ips.lock().await.push(result);
                    
                    
                }
                println!("Done!");

                println!("Pingable IPs: {:?}", *pingable_ips.lock().await);

                for (i,pingable_ip) in pingable_ips.lock().await.iter().enumerate() {
                    println!("{}: online", pingable_ip);
                    pingable_ips_json.push(
                        json!({
                            "id": i,
                            "name": pingable_ip.to_string()
                        })
                    );
                }
            }
        }
    }
    return pingable_ips_json;

}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_network_interfaces, list_ips])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
