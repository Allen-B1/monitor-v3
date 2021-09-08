mod process;
use std::{borrow::Borrow, collections::HashMap, env::args, error::Error, hash::Hash};

use monitor::http::{Device, DeviceData, DeviceID};
use tokio::time;
extern crate clap;
use clap::App;

#[test]
fn test_active() {
    let active_id = process::get_active_window().unwrap();
    let data = process::get_window_info(active_id).unwrap().unwrap();
    println!("id = {}", active_id);
    println!("program = {}", &data.program);
    println!("title = {}", &data.title);
}

fn get_device_id() -> Option<DeviceID> {
    use std::net::{UdpSocket, IpAddr};

    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    match socket.connect("8.8.8.8:80") {
        Ok(()) => (),
        Err(_) => return None,
    };

    let ip = match socket.local_addr() {
        Ok(addr) => addr.ip(),
        Err(_) => return None,
    };

    match ip {
        IpAddr::V4(v4) => {
            Some(v4.octets()[3] as DeviceID)
        },
        IpAddr::V6(v6) => {
            Some(v6.segments()[7] as DeviceID)
        }
    }
}

#[test]
fn test_device_id() {
    println!("{:?}", get_device_id());
}

fn get_device_info() -> Result<monitor::http::DeviceData, Box<dyn Error>> {
    let release_data = os_release::OsRelease::new()?;

    Ok(monitor::http::DeviceData {
        type_ : monitor::http::DeviceType::Laptop,
        os: "Linux".to_owned(),
        distro: Some(release_data.pretty_name),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = clap::App::new("monitor-linux")
        .about("monitor client for X11 / Linux")
        .arg(clap::Arg::with_name("name")
            .short("n")
            .long("name")
            .value_name("NAME")
            .help("Your name, given to the monitor server")
            .takes_value(true)
            .required(true))
        .arg(clap::Arg::with_name("device-id")
            .short("d")
            .long("device-id")
            .value_name("ID")
            .help("Device ID of this computer")
            .takes_value(true)
            .required(false))
        .get_matches();

    let name = matches.value_of("name").unwrap();
    let device_id = matches.value_of("device-id").map(<DeviceID as std::str::FromStr>::from_str).and_then(Result::ok).unwrap_or_else(|| get_device_id().unwrap());

    let mut interval = time::interval(time::Duration::from_secs(1));
    let client = reqwest::Client::new();
    let mut seconds = 0;
    let mut http_data: monitor::http::Add = monitor::http::Add::new(device_id);

    println!("username: {}", name);
    println!("device id: {}", device_id);

    client.post(format!("http://127.0.0.1:7246/api/{}/device", name)).json(&monitor::http::Device {
        id: device_id,
        data: get_device_info().unwrap()
    }).send().await?;

    loop {
        let active_id = process::get_active_window()?;
        let windows = process::get_all_windows()?;
        for id in windows {
            let data = process::get_window_info(id)?;
            if let Some(data) = data {
                if id == active_id {
                    *http_data.active.entry(data.clone().into()).or_insert(0) += 1;
                }
                *http_data.open.entry(data.into()).or_insert(0) += 1;
            }
        }

        if seconds > 15 {
            seconds = 0;
            client.post(format!("http://127.0.0.1:7246/api/{}/add", name)).json(&http_data).send().await?;
            http_data = monitor::http::Add::new(device_id);
        }
        seconds += 1;

        interval.tick().await;
    }
}

