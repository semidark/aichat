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
use rocket::serde::{Deserialize, Serialize, json::Json};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::config::{Config, GlobalConfig, WorkingMode};

type AppState = Arc<RwLock<Config>>;

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct ChatRequest {
    message: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct ChatResponse {
    response: String,
    status: String,
}

#[post("/chat", data = "<chat_request>")]
fn chat(chat_request: Json<ChatRequest>, _state: &State<AppState>) -> Json<ChatResponse> {
    // For now, return a simple echo response
    // This will be expanded in subsequent tasks to include:
    // - Session handling (Task 2.2)
    // - Conversation history loading (Task 2.3) 
    // - LLM integration (Task 2.4)
    // - Session file updates (Task 2.5)
    
    let user_message = &chat_request.message;
    let response_text = format!("Echo: {}", user_message);
    
    Json(ChatResponse {
        response: response_text,
        status: "success".to_string(),
    })
}

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
        .mount("/api", routes![chat])
        .mount("/", FileServer::from(relative!("static")))
}
