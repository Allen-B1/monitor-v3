mod process;
use std::{borrow::Borrow, collections::HashMap, env::args, hash::Hash};

use monitor::http;
use tokio::time;

#[test]
fn test_active() {
    let active_id = process::get_active_window().unwrap();
    let data = process::get_window_info(active_id).unwrap().unwrap();
    println!("id = {}", active_id);
    println!("program = {}", &data.program);
    println!("title = {}", &data.title);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let mut interval = time::interval(time::Duration::from_secs(1));
    let mut seconds = 0;

    let mut http_data: monitor::http::Add = Default::default();

    let name = args().skip(1).next().ok_or("Must have a name")?;

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
            http_data = Default::default();
        }
        seconds += 1;

        interval.tick().await;
    }
}

