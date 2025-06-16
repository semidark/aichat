#[macro_use] extern crate rocket;

mod cli;
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

// Add imports for LLM integration
use crate::client::call_chat_completions;
use crate::config::{Config, WorkingMode, GlobalConfig, Input};
use crate::utils::create_abort_signal;
use anyhow::Result;

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

    // Convert conversation history to a single text string for LLM input
    fn to_conversation_text(&self) -> String {
        if self.messages.is_empty() {
            return String::new();
        }
        
        let mut conversation_parts = Vec::new();
        
        // Add all previous messages as context
        for (i, msg) in self.messages.iter().enumerate() {
            if i == self.messages.len() - 1 {
                // Skip the last message as it's the current user input
                break;
            }
            
            let role_prefix = match msg.role.as_str() {
                "user" => "Human",
                "assistant" => "Assistant",
                _ => &msg.role,
            };
            conversation_parts.push(format!("{}: {}", role_prefix, msg.content));
        }
        
        // Add the current user message
        if let Some(last_msg) = self.messages.last() {
            if last_msg.role == "user" {
                conversation_parts.push(last_msg.content.clone());
            }
        }
        
        conversation_parts.join("\n\n")
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
async fn chat(
    chat_request: Json<ChatRequest>, 
    cookies: &CookieJar<'_>,
    state: &State<AppState>
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
    
    // Create the global config for LLM integration
    let global_config = state.inner().clone();
    
    // Create Input from conversation history
    let conversation_text = history.to_conversation_text();
    let input = Input::from_str(&global_config, &conversation_text, None);
    
    // Create abort signal for the LLM call
    let abort_signal = create_abort_signal();
    
    // Call the LLM
    let response_text = match call_llm(&input, &global_config, abort_signal).await {
        Ok(text) => text,
        Err(e) => {
            eprintln!("Error calling LLM: {}", e);
            format!("Sorry, I encountered an error: {}", e)
        }
    };
    
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

// Helper function to call the LLM using aichat's client system
async fn call_llm(
    input: &Input,
    global_config: &GlobalConfig,
    abort_signal: crate::utils::AbortSignal,
) -> Result<String> {
    // Create client from input
    let client = input.create_client()?;
    
    // Prepare for chat completion
    global_config.write().before_chat_completion(input)?;
    
    // Call the LLM (non-streaming for now, as per task 2.5 requirement)
    let (output, tool_results) = call_chat_completions(
        input,
        false, // don't print to stdout
        false, // don't extract code
        client.as_ref(),
        abort_signal,
    ).await?;
    
    // Handle completion
    global_config.write().after_chat_completion(input, &output, &tool_results)?;
    
    Ok(output)
}

#[get("/")]
fn index() -> &'static str {
    "Hello, Kindle AI Chat!"
}

/// Create and configure the Rocket instance for the Kindle AI Chat server.
/// This function can be used both for launching the server and for testing.
pub async fn rocket() -> rocket::Rocket<rocket::Build> {
    // Initialize configuration for web server mode
    let config = Config::init(WorkingMode::Serve, false).await
        .expect("Failed to initialize config");
    let app_state: AppState = Arc::new(RwLock::new(config));

    rocket::build()
        .manage(app_state)
        .mount("/api", routes![chat])
        .mount("/", FileServer::from(relative!("static")))
}

#[tokio::main]
async fn main() -> Result<()> {
    use crate::cli::Cli;
    use clap::Parser;
    
    // Parse CLI arguments
    let cli = Cli::parse();
    
    // Check if this is a CLI command (like --list-models) or server mode
    let is_cli_command = cli.list_models || cli.list_roles || cli.list_sessions || 
                        cli.list_agents || cli.list_rags || cli.list_macros ||
                        cli.info || cli.sync_models;
    
    if is_cli_command {
        // Run original CLI functionality
        run_cli(cli).await
    } else {
        // Run Rocket server
        run_server().await
    }
}

async fn run_cli(cli: crate::cli::Cli) -> Result<()> {
    // Import necessary items for CLI functionality
    use crate::client::{list_models, ModelType};
    use crate::config::{Config, WorkingMode};
    use parking_lot::RwLock;
    use std::sync::Arc;
    
    // Initialize config for CLI mode
    let working_mode = if cli.serve.is_some() {
        WorkingMode::Serve
    } else {
        WorkingMode::Cmd
    };
    
    let config = Arc::new(RwLock::new(Config::init(working_mode, true).await?));
    
    // Handle CLI commands
    if cli.list_models {
        for model in list_models(&config.read(), ModelType::Chat) {
            println!("{}", model.id());
        }
        return Ok(());
    }
    
    if cli.list_roles {
        let roles = Config::list_roles(true).join("\n");
        println!("{roles}");
        return Ok(());
    }
    
    if cli.info {
        let info = config.read().info()?;
        println!("{}", info);
        return Ok(());
    }
    
    // Add other CLI commands as needed
    println!("CLI command not yet implemented in Kindle AI Chat fork");
    Ok(())
}

async fn run_server() -> Result<()> {
    rocket().await.launch().await.map_err(|e| anyhow::anyhow!("Rocket error: {}", e))?;
    Ok(())
}
