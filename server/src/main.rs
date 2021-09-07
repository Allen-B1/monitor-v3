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

#[tokio::main]
async fn main() {
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