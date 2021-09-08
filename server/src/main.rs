#[macro_use]
extern crate lazy_static;

use std::{collections::HashMap, sync::Mutex};

use warp::{Filter, Rejection, Reply};
use serde::{Serialize,Deserialize};

#[derive(Clone, Default, Serialize, Deserialize)]
struct MonitorData {
    active: HashMap<monitor::ActiveProgram, u32>,
    open: HashMap<monitor::Program, u32>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct UserData {
    /// Not every valid device is guaranteed to have a `DeviceData`.
    devices: HashMap<monitor::http::DeviceID, monitor::http::DeviceData>,
    
    
    monitor: HashMap<monitor::http::DeviceID, MonitorData>
}

lazy_static! {
    static ref STATIC_DATA: Mutex<HashMap<String, UserData>> = Mutex::new(HashMap::new());
}

#[tokio::main]
async fn main() {
    let save_file = format!("data-{}.json", chrono::Local::today().naive_local().format("%Y-%m-%d"));
    match std::fs::File::open(&save_file) {
        Ok(f) => {
            *STATIC_DATA.lock().unwrap() = serde_json::from_reader(f).unwrap();
        },
        Err(err) => {
            eprintln!("save file '{}' not found, creating later", &save_file);
        }
    };

    // save STATIC_DATA periodically
    tokio::spawn(async {
        let mut date = chrono::Local::today().naive_local();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;

            let save_file = format!("data-{}.json", date.format("%Y-%m-%d"));
            let mut file = match std::fs::File::create(&save_file) {
                Ok(f) => f,
                Err(err) => {
                    eprintln!("error saving file: {}", err);
                    continue
                }
            };

            let mut data = STATIC_DATA.lock().unwrap();
            serde_json::to_writer(&mut file, &*data);

            let new_date = chrono::Local::today().naive_local();
            if date != new_date {
                date = new_date;
                *data = Default::default();
            }
        }
    });
    
    let api_add = warp::path!("api" / String / "add")
        .and(warp::post())
        .and(warp::body::json())
        .map(|name: String, body: monitor::http::Add| {
            let mut data = STATIC_DATA.lock().unwrap();
            let data = data.entry(name).or_default();            
            let data = data.monitor.entry(body.device).or_default();

            for (active, &secs) in &body.active {
                *data.active.entry(active.clone()).or_insert(0) += secs;
            }
            for (open, &secs) in &body.open {
                *data.open.entry(open.clone()).or_insert(0) += secs;
            }

            println!("req recieved");

            warp::reply::json(&())
        });
    
    let api_device = warp::path!("api" / String / "device")
        .and(warp::post())
        .and(warp::body::json())
        .map(|name: String, body: monitor::http::Device| {
            let mut data = STATIC_DATA.lock().unwrap();
            let data = data.entry(name).or_default();
            let data = data.devices.entry(body.id).or_default();
            *data = body.data;

            warp::reply::json(&())
        });
    
    let api_today = warp::path!("api" / String / "today")
        .and(warp::get())
        .map(|name: String| {
            let mut data = STATIC_DATA.lock().unwrap();
            warp::reply::json(&data.get(&name).cloned())
        });

    let routes = api_today
        .or(api_add)
        .or(api_device)
        .recover(error_func);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 7246))
        .await;
}

async fn error_func(rejection: Rejection) -> Result<warp::reply::Json, Rejection> { 
    eprintln!("error: {:?}", rejection);
    
    Err(rejection)
}