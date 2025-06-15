#[macro_use] extern crate rocket;

mod client;
mod config;
mod function;
mod rag;
mod render;
mod repl;
mod serve;
#[macro_use]
mod utils;

use rocket::fs::{FileServer, relative};
use rocket::{State, Rocket, Build};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::config::{Config, GlobalConfig, WorkingMode};

type AppState = Arc<RwLock<Config>>;

#[get("/")]
fn index() -> &'static str {
    "Hello, Kindle AI Chat!"
}

#[launch]
fn rocket() -> _ {
    // Initialize configuration for web server mode
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let config = rt.block_on(async {
        Config::init(WorkingMode::Serve, false).await
            .expect("Failed to initialize config")
    });
    
    let app_state: AppState = Arc::new(RwLock::new(config));

    rocket::build()
        .manage(app_state)
        .mount("/", routes![index])
        .mount("/static", FileServer::from(relative!("static")))
}
