#[macro_use]
extern crate lazy_static;

use std::{collections::HashMap, error::Error, sync::Mutex, task::Context};

use chrono::{Datelike, NaiveDate};
use monitor::http;
use serde_json::json;
use std::borrow::Borrow;
use warp::{Filter, Rejection, Reply};
use serde::{Serialize,Deserialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct MonitorData {
    active: HashMap<monitor::ActiveProgram, u32>,
    open: HashMap<monitor::Program, u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct UserData {
    /// Not every valid device is guaranteed to have a `DeviceData`.
    devices: HashMap<monitor::http::DeviceID, monitor::http::DeviceData>,
    
    
    monitor: HashMap<monitor::http::DeviceID, MonitorData>
}

lazy_static! {
    static ref STATIC_DATA: Mutex<HashMap<String, UserData>> = Mutex::new(HashMap::new());
}

#[derive(Debug)]
pub struct RejectGeneric(String);
impl warp::reject::Reject for RejectGeneric {}

#[tokio::main]
async fn main() {
    let args = clap::App::new("monitor-server")
        .about("server for monitor")
        .arg(clap::Arg::with_name("port")
            .short("p")
            .long("port")
            .takes_value(true)
            .value_name("NUM")
            .help("HTTP port to serve on")
            .default_value("7246")
        )
        .get_matches();
    
    let port = args.value_of("port").unwrap().parse::<u16>().unwrap();

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

    let page_device = warp::path!(String / u32 / u8 / u8 / u16)
        .and(warp::get())
        .and_then(handle_page_device);

    let page_redirect = warp::path!(String / "redirect")
        .and(warp::get())
        .and(warp::query())
        .and_then(handle_page_redirect);

    let page_person = warp::path!(String)
        .and(warp::get())
        .map(|name| {
            let date = chrono::Local::now().naive_local().date();
            let data = STATIC_DATA.lock().unwrap();
            warp::redirect::temporary(format!("/{}/{}/{}/{}/{}", name, date.year(), date.month(), date.day(),
                data.get(&name)
                    .and_then(|data| data.monitor.keys().next())
                    .map(|&a| a)
                    .unwrap_or(0)
            ).parse::<warp::http::Uri>().unwrap())
        });

    let routes = api_today
        .or(api_add)
        .or(api_device)
        .or(page_device)
        .or(page_redirect)
        .or(page_person)
        .recover(error_func);

    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;
}

async fn handle_page_redirect (name: String, query: HashMap<String, String>) -> Result<impl Reply, Rejection> {
    let date_str = query.get("date").ok_or(warp::reject::not_found())?;
    let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|e| RejectGeneric(e.to_string()))?;
    let device = query.get("device").ok_or(warp::reject::not_found())?.parse::<monitor::http::DeviceID>().map_err(|e| RejectGeneric(e.to_string()))?;

    Ok(warp::redirect::redirect(format!("/{}/{}/{}/{}/{}", name, date.year(), date.month(), date.day(), device).parse::<warp::http::Uri>().unwrap()))
}

async fn error_func(rejection: Rejection) -> Result<warp::reply::Html<String>, Rejection> { 
    eprintln!("error: {:?}", rejection);

    // TODO: error page
    Ok(warp::reply::html(format!("error: {:?}", rejection)))
}

#[derive(Debug)]
pub struct RejectBadTemplate(String);
impl warp::reject::Reject for RejectBadTemplate {}
impl<E: Error> From<E> for RejectBadTemplate {
    fn from(e: E) -> Self {
        Self(e.to_string())
    }
}
#[derive(Debug)]
pub struct RejectBadData(String);
impl warp::reject::Reject for RejectBadData {}
impl<E: Error> From<E> for RejectBadData {
    fn from(e: E) -> Self {
        Self(e.to_string())
    }
}

#[litem::template("server/templates/header.html", escape="html")]
struct HeaderTemplate {
    name: String,
    date: NaiveDate,
    device: monitor::http::DeviceID,
    devices: HashMap<monitor::http::DeviceID, monitor::http::DeviceData>,
}

#[litem::template("server/templates/no-data.html", escape="html")]
struct NoDataTemplate {
    name: String,
    date: NaiveDate,
    device: monitor::http::DeviceID,
    devices: HashMap<monitor::http::DeviceID, monitor::http::DeviceData>,
}

#[litem::template("server/templates/data.html", escape="html")]
struct DataTemplate {
    name: String,
    date: NaiveDate,
    device: monitor::http::DeviceID,
    devices: HashMap<monitor::http::DeviceID, monitor::http::DeviceData>,

    monitor: MonitorData,
    active_data: HashMap<String, (u32, Vec<String>)>,
}

async fn handle_page_device(name: String, year: u32, month: u8, day: u8, device: monitor::http::DeviceID) -> Result<Box<dyn warp::reply::Reply>, Rejection> {
    let today = chrono::Local::now().naive_local().date();
    let date = chrono::NaiveDate::from_ymd(year as i32, month.into(), day.into());

    let no_data = || {
        warp::reply::html(NoDataTemplate {
            name: name.clone(),
            date: date.clone(),
            device: device,
            devices: {
                let mut h=  HashMap::new();
                h.insert(device, monitor::http::DeviceData { type_: Default::default(), distro: Some(device.to_string()), os: "Unknown".to_owned()});
                h
            }
        }.render_string().unwrap())
    };

    let data = if today == date {
        let data = STATIC_DATA.lock().unwrap();
        data.clone()
    } else {
        let data = match std::fs::read_to_string(format!("data-{}.json", date.format("%Y-%m-%d"))) {
            Ok(v) => v,
            Err(e) => return Ok(Box::new(no_data()))
        };
        let data: HashMap<String, UserData> = match serde_json::from_str(&data) {
            Ok(v) => v,
            Err(e) => return Ok(Box::new(no_data()))
        };
        data
    };

    let data = match data.get(&name) {
        Some(v) => v,
        None => return Ok(Box::new(no_data()))
    };
    let monitor = match data.monitor.get(&device) {
        Some(v) => v,
        None => return Ok(Box::new(no_data()))
    };

    let mut active_data: HashMap<String, (u32, Vec<String>)> = HashMap::new();
    for (program, &time) in monitor.active.iter() {
        let entry = active_data.entry(program.program.to_owned()).or_default();
        entry.0 += time;

        if let Some(s) = program.subprogram.as_ref() {
            entry.1.push(s.clone());
        }
    }


    let reply = DataTemplate {
        name,
        date,
        device,
        devices: data.devices.clone(),
        monitor: monitor.clone(), active_data
    }.render_string().map_err(|e| warp::reject::custom(RejectBadTemplate(e.to_string())))?;
    Ok(Box::new(warp::reply::html(reply)))
}