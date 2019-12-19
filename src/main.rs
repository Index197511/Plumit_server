extern crate ctrlc;
extern crate sled;

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use serde_derive::{Deserialize, Serialize};
use sled::Db;

type Weight = (String, String);

#[derive(Debug, Serialize, Deserialize)]
struct SentWeightData {
    token: String,
    date: String,
    time: String,
    total: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Crew {
    number_of_visitors: i32,
    body_weight: Vec<WeightData>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WeightData {
    date: String,
    weight: f32,
}

struct NumberOfVisitorsCounter {
    number_of_visitors: Mutex<i32>,
}

fn from_utf8s(weight_ivec: (sled::IVec, sled::IVec)) -> Option<Weight> {
    let data_utf8 = weight_ivec.0.to_vec();
    let weight_utf8 = weight_ivec.1.to_vec();

    match (String::from_utf8(data_utf8), String::from_utf8(weight_utf8)) {
        (Ok(data), Ok(weight)) => Some((data, weight)),
        _ => None,
    }
}

fn show(visit_counter: web::Data<NumberOfVisitorsCounter>) -> HttpResponse {
    let db = Db::open("/home/index197511/body_weight_meter/weight_storage").unwrap();
    let mut return_data: Vec<WeightData> = Vec::new();

    let mut counter = visit_counter.number_of_visitors.lock().unwrap();
    *counter += 1;

    let weight_utf8 = db
        .iter()
        .filter(|maybe_weight| maybe_weight.is_ok())
        .map(|ok_weight| ok_weight.unwrap());

    for converted_weight in weight_utf8.map(|w| from_utf8s(w)) {
        if let Some(c) = converted_weight {
            return_data.push(WeightData {
                date: c.0,
                weight: c.1.parse::<f32>().unwrap(),
            });
        }
    }

    let ret = Crew {
        body_weight: return_data,
        number_of_visitors: *counter,
    };

    HttpResponse::Ok().json(ret)
}

fn registration(item: web::Json<SentWeightData>) {
    let token = fs::read_to_string("/home/index197511/body_weight_meter/src/token.txt")
        .unwrap()
        .replace("\n", "");

    println!("default token is {:?}", token);
    println!("sent token is {:?}", item.token);

    if token != item.token {
        println!("Token is not same!");
        return;
    }

    println!("Token is same!");

    println!("sent data is {:?}", item);

    let db = Db::open("weight_storage").unwrap();
    let key = format!("{}/{}", item.date, item.time);

    let _ = db.insert(&key, item.total.as_bytes().to_vec());
}

fn delete_db(item: web::Json<SentWeightData>) {
    let token = fs::read_to_string("/home/index197511/body_weight_meter/src/token.txt")
        .unwrap()
        .replace("\n", "");
    if token != item.token {
        return;
    }

    let db = Db::open("weight_storage").unwrap();
    let _ = db.clear();
}

fn app_config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("")
            .service(web::resource("/show").route(web::get().to(show)))
            .service(web::resource("/post").route(web::post().to(registration)))
            .service(web::resource("/delete").route(web::post().to(delete_db))),
    );
}

fn write(write_obj: web::Data<NumberOfVisitorsCounter>) {
    let counter = write_obj.number_of_visitors.lock().unwrap();
    let _ = fs::write(
        "/home/index197511/body_weight_meter/visitor.txt",
        (*counter).to_string(),
    );
}

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let read_counter: i32 = fs::read_to_string("/home/index197511/body_weight_meter/visitor.txt")
        .unwrap()
        .replace("\n", "")
        .parse::<i32>()
        .unwrap();

    let counter = web::Data::new(NumberOfVisitorsCounter {
        number_of_visitors: Mutex::new(read_counter),
    });

    let register_counter = counter.clone();

    HttpServer::new(move || {
        App::new()
            .register_data(counter.clone())
            .wrap(middleware::Logger::default())
            .data(web::JsonConfig::default().limit(4096))
            .configure(app_config)
    })
    .bind("127.0.0.1:10080")
    .unwrap()
    .run()
    .unwrap();

    write(register_counter);
}
