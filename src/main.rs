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
use rocket::http::{Cookie, CookieJar, SameSite};
use std::sync::Arc;
use std::fs;
use std::path::Path;
use parking_lot::RwLock;
use uuid::Uuid;
use chrono;

use crate::config::{Config, WorkingMode};

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

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
struct ConversationMessage {
    role: String,    // "user" or "assistant"
    content: String,
    timestamp: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct ConversationHistory {
    session_id: String,
    messages: Vec<ConversationMessage>,
    created_at: i64,
    updated_at: i64,
}

impl ConversationHistory {
    fn new(session_id: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            session_id,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    fn add_message(&mut self, role: String, content: String) {
        let timestamp = chrono::Utc::now().timestamp();
        self.messages.push(ConversationMessage {
            role,
            content,
            timestamp,
        });
        self.updated_at = timestamp;
    }

    fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let data_dir = Path::new("data");
        if !data_dir.exists() {
            fs::create_dir_all(data_dir)?;
        }
        
        let file_path = data_dir.join(format!("{}.json", self.session_id));
        let json_content = serde_json::to_string_pretty(self)?;
        fs::write(file_path, json_content)?;
        Ok(())
    }

    fn load_from_file(session_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file_path = Path::new("data").join(format!("{}.json", session_id));
        if !file_path.exists() {
            return Ok(Self::new(session_id.to_string()));
        }
        
        let json_content = fs::read_to_string(file_path)?;
        let mut history: ConversationHistory = serde_json::from_str(&json_content)?;
        
        // Update session_id in case it doesn't match (shouldn't happen, but safety check)
        history.session_id = session_id.to_string();
        Ok(history)
    }
}

fn get_or_create_session_id(cookies: &CookieJar<'_>) -> String {
    // Try to get existing session ID from cookie
    if let Some(cookie) = cookies.get("session_id") {
        if let Ok(uuid) = Uuid::parse_str(cookie.value()) {
            return uuid.to_string();
        }
    }
    
    // Generate new session ID
    let session_id = Uuid::new_v4().to_string();
    
    // Set persistent cookie (expires in 30 days)
    let mut cookie = Cookie::new("session_id", session_id.clone());
    cookie.set_max_age(rocket::time::Duration::days(30));
    cookie.set_same_site(SameSite::Lax);
    cookie.set_http_only(true);
    cookies.add(cookie);
    
    session_id
}

#[post("/chat", data = "<chat_request>")]
fn chat(
    chat_request: Json<ChatRequest>, 
    cookies: &CookieJar<'_>,
    _state: &State<AppState>
) -> Json<ChatResponse> {
    // Get or create session ID
    let session_id = get_or_create_session_id(cookies);
    
    // Load conversation history
    let mut history = match ConversationHistory::load_from_file(&session_id) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Error loading conversation history: {}", e);
            ConversationHistory::new(session_id.clone())
        }
    };
    
    // Add user message to history
    let user_message = &chat_request.message;
    history.add_message("user".to_string(), user_message.clone());
    
    // For now, create a simple response that includes conversation context
    let message_count = history.messages.len();
    let response_text = format!(
        "Echo (Session: {}, Message #{}: {})", 
        &session_id[..8], // Show first 8 chars of UUID
        message_count,
        user_message
    );
    
    // Add assistant response to history
    history.add_message("assistant".to_string(), response_text.clone());
    
    // Save updated history
    if let Err(e) = history.save_to_file() {
        eprintln!("Error saving conversation history: {}", e);
    }
    
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
