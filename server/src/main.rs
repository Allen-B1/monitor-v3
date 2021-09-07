#[macro_use]
extern crate lazy_static;

use std::{collections::HashMap, sync::Mutex};

use warp::{Filter, Rejection, Reply};
use serde::{Serialize,Deserialize};

#[derive(Clone, Default, Serialize, Deserialize)]
struct Data {
    active: HashMap<monitor::ActiveProgram, u32>,
    open: HashMap<monitor::Program, u32>,
}

lazy_static! {
    static ref STATIC_DATA: Mutex<HashMap<String, Data>> = Mutex::new(HashMap::new());
}

const SAVE_FILE: &'static str = "data.json";

#[tokio::main]
async fn main() {
    // save STATIC_DATA periodically
    tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;

            let mut file = match std::fs::File::create(SAVE_FILE) {
                Ok(f) => f,
                Err(err) => {
                    eprintln!("error saving file: {}", err);
                    continue
                }
            };
            serde_json::to_writer(&mut file, &*STATIC_DATA.lock().unwrap());
        }
    });

    match std::fs::File::open(SAVE_FILE) {
        Ok(f) => {
            *STATIC_DATA.lock().unwrap() = serde_json::from_reader(f).unwrap();
        },
        Err(err) => {
            eprintln!("save file '{}' not found, creating later", SAVE_FILE);
        }
    };

    let api_add = warp::path!("api" / String / "add")
        .and(warp::post())
        .and(warp::body::json())
        .map(|name: String, body: monitor::http::Add| {
            let mut data = STATIC_DATA.lock().unwrap();
            let data = data.entry(name).or_default();
            
            for (active, &secs) in &body.active {
                *data.active.entry(active.clone()).or_insert(0) += secs;
            }
            for (open, &secs) in &body.open {
                *data.open.entry(open.clone()).or_insert(0) += secs;
            }

            println!("req recieved");

            warp::reply::json(&())
        });
    
    let api_get = warp::path!("api" / String / "get")
        .and(warp::get())
        .map(|name: String| {
            let mut data = STATIC_DATA.lock().unwrap();
            warp::reply::json(&data.get(&name).cloned())
        });

    let routes = api_get.or(api_add).recover(error_func);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 7246))
        .await;
}

async fn error_func(rejection: Rejection) -> Result<warp::reply::Json, Rejection> { 
    eprintln!("error: {:?}", rejection);
    
    Err(rejection)
}