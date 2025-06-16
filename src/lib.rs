//! Kindle AI Chat - A lightweight AI chat interface optimized for Kindle e-readers
//! 
//! This library provides the core functionality for the Kindle AI Chat application,
//! including web server setup, session management, and LLM integration.

#[macro_use] 
extern crate rocket;

// Re-export modules from the original aichat codebase
pub mod cli;
pub mod client;
pub mod config;
pub mod function;
pub mod rag;
pub mod render;
pub mod repl;
pub mod serve;
#[macro_use]
pub mod utils;

// Rocket and web-related imports
use rocket::fs::{FileServer, relative};
use rocket::State;
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::form::{Form, FromForm};
use rocket::response::content::RawHtml;

// Standard library imports
use std::sync::Arc;
use std::fs;
use std::path::Path;

// External crate imports
use parking_lot::RwLock;
use uuid::Uuid;
use chrono;
use anyhow::Result;
use serde::{Serialize, Deserialize};

// Internal imports for LLM integration
use crate::client::call_chat_completions;
use crate::config::{Config, WorkingMode, GlobalConfig, Input};
use crate::utils::create_abort_signal;

/// Application state type alias for cleaner code
pub type AppState = Arc<RwLock<Config>>;

/// Request structure for chat endpoint (Form data)
#[derive(FromForm)]
pub struct ChatForm {
    pub message: String,
}

/// Individual message in a conversation
#[derive(Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,    // "user" or "assistant"
    pub content: String,
    pub timestamp: i64,
}

/// Complete conversation history for a session
#[derive(Serialize, Deserialize)]
pub struct ConversationHistory {
    pub session_id: String,
    pub messages: Vec<ConversationMessage>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl ConversationHistory {
    /// Create a new conversation history for a session
    pub fn new(session_id: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            session_id,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a message to the conversation history
    pub fn add_message(&mut self, role: String, content: String) {
        let timestamp = chrono::Utc::now().timestamp();
        self.messages.push(ConversationMessage {
            role,
            content,
            timestamp,
        });
        self.updated_at = timestamp;
    }

    /// Save conversation history to a JSON file
    pub fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let data_dir = Path::new("data");
        if !data_dir.exists() {
            fs::create_dir_all(data_dir)?;
        }
        
        let file_path = data_dir.join(format!("{}.json", self.session_id));
        let json_content = serde_json::to_string_pretty(self)?;
        fs::write(file_path, json_content)?;
        Ok(())
    }

    /// Load conversation history from a JSON file
    pub fn load_from_file(session_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
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

    /// Convert conversation history to a single text string for LLM input
    pub fn to_conversation_text(&self) -> String {
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

/// Get existing session ID from cookies or create a new one
pub fn get_or_create_session_id(cookies: &CookieJar<'_>) -> String {
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

/// Main chat endpoint handler for htmx form submission (returns HTML)
#[post("/chat", data = "<chat_form>")]
pub async fn chat(
    chat_form: Form<ChatForm>, 
    cookies: &CookieJar<'_>,
    state: &State<AppState>
) -> RawHtml<String> {
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
    let user_message = &chat_form.message;
    history.add_message("user".to_string(), user_message.clone());
    
    // HTML escape function for security
    let html_escape = |s: &str| {
        s.replace('&', "&amp;")
         .replace('<', "&lt;")
         .replace('>', "&gt;")
         .replace('"', "&quot;")
         .replace('\'', "&#x27;")
    };
    
    // Note: User message HTML is handled by JavaScript for immediate display
    // We only return the assistant message from the server
    
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
    
    // Create assistant message HTML
    let assistant_html = format!(
        r#"<div class="message assistant">
            <div class="message-role">Assistant:</div>
            <div class="message-content">{}</div>
        </div>"#,
        html_escape(&response_text)
    );
    
    // Return only assistant message as HTML
    // (User message is already displayed immediately by JavaScript for better UX)
    RawHtml(assistant_html)
}

/// Helper function to call the LLM using aichat's client system
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

/// Basic index route for testing
#[get("/")]
pub fn index() -> &'static str {
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

/// Run the CLI functionality (original aichat commands)
pub async fn run_cli(cli: crate::cli::Cli) -> Result<()> {
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

/// Run the Rocket web server
pub async fn run_server() -> Result<()> {
    rocket().await.launch().await.map_err(|e| anyhow::anyhow!("Rocket error: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper function to create a temporary directory for test files
    fn create_temp_data_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp directory")
    }

    /// Test ConversationHistory creation and basic functionality
    #[test]
    fn test_conversation_history_new() {
        let session_id = "test-session-123".to_string();
        let history = ConversationHistory::new(session_id.clone());
        
        assert_eq!(history.session_id, session_id);
        assert!(history.messages.is_empty());
        assert!(history.created_at > 0);
        assert_eq!(history.created_at, history.updated_at);
    }

    /// Test adding messages to conversation history
    #[test]
    fn test_conversation_history_add_message() {
        let mut history = ConversationHistory::new("test-session".to_string());
        
        history.add_message("user".to_string(), "Hello, world!".to_string());
        assert_eq!(history.messages.len(), 1);
        assert_eq!(history.messages[0].role, "user");
        assert_eq!(history.messages[0].content, "Hello, world!");
        
        history.add_message("assistant".to_string(), "Hi there!".to_string());
        assert_eq!(history.messages.len(), 2);
        assert_eq!(history.messages[1].role, "assistant");
        assert_eq!(history.messages[1].content, "Hi there!");
    }

    /// Test conversation text formatting for LLM
    #[test]
    fn test_to_conversation_text() {
        let mut history = ConversationHistory::new("test-session".to_string());
        
        // Test empty conversation
        assert_eq!(history.to_conversation_text(), "");
        
        // Test single user message
        history.add_message("user".to_string(), "What is Rust?".to_string());
        assert_eq!(history.to_conversation_text(), "What is Rust?");
        
        // Test conversation with assistant response
        history.add_message("assistant".to_string(), "Rust is a systems programming language.".to_string());
        history.add_message("user".to_string(), "Tell me more.".to_string());
        
        let expected = "Human: What is Rust?\n\nAssistant: Rust is a systems programming language.\n\nTell me more.";
        assert_eq!(history.to_conversation_text(), expected);
    }

    /// Test saving and loading conversation history to/from file
    /// This test verifies the complete round-trip persistence functionality
    #[test]
    fn test_conversation_history_save_and_load() {
        let temp_dir = create_temp_data_dir();
        let original_cwd = std::env::current_dir().unwrap();
        
        // Change to temp directory so "data" directory gets created there
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Create and populate conversation history with multiple message types
        let session_id = "test-save-load-comprehensive".to_string();
        let mut original_history = ConversationHistory::new(session_id.clone());
        
        // Add various types of messages to test comprehensive serialization
        original_history.add_message("user".to_string(), "Hello, AI assistant!".to_string());
        original_history.add_message("assistant".to_string(), "Hello! How can I help you today?".to_string());
        original_history.add_message("user".to_string(), "What's the weather like?".to_string());
        original_history.add_message("assistant".to_string(), "I don't have access to real-time weather data, but I can help you find weather information.".to_string());
        
        // Record original timestamps for verification
        let original_created_at = original_history.created_at;
        let original_updated_at = original_history.updated_at;
        let original_message_count = original_history.messages.len();
        
        // Save to file
        original_history.save_to_file().expect("Failed to save conversation history");
        
        // Verify the file was actually created
        let data_dir = std::path::Path::new("data");
        let file_path = data_dir.join(format!("{}.json", session_id));
        assert!(file_path.exists(), "Conversation history file should exist after saving");
        
        // Verify the file contains valid JSON
        let file_content = std::fs::read_to_string(&file_path).expect("Should be able to read saved file");
        assert!(!file_content.is_empty(), "Saved file should not be empty");
        assert!(file_content.contains(&session_id), "File should contain session ID");
        assert!(file_content.contains("Hello, AI assistant!"), "File should contain user message");
        
        // Load from file
        let loaded_history = ConversationHistory::load_from_file(&session_id)
            .expect("Failed to load conversation history");
        
        // Verify all data matches between original and loaded versions
        assert_eq!(loaded_history.session_id, original_history.session_id, "Session ID should match");
        assert_eq!(loaded_history.messages.len(), original_message_count, "Message count should match");
        assert_eq!(loaded_history.created_at, original_created_at, "Created timestamp should match");
        assert_eq!(loaded_history.updated_at, original_updated_at, "Updated timestamp should match");
        
        // Verify individual messages
        for (i, (original_msg, loaded_msg)) in original_history.messages.iter().zip(loaded_history.messages.iter()).enumerate() {
            assert_eq!(loaded_msg.role, original_msg.role, "Message {} role should match", i);
            assert_eq!(loaded_msg.content, original_msg.content, "Message {} content should match", i);
            assert_eq!(loaded_msg.timestamp, original_msg.timestamp, "Message {} timestamp should match", i);
        }
        
        // Test that we can load the same file multiple times consistently
        let loaded_again = ConversationHistory::load_from_file(&session_id)
            .expect("Should be able to load the same file multiple times");
        assert_eq!(loaded_again.messages.len(), loaded_history.messages.len(), "Multiple loads should be consistent");
        
        // Restore original directory
        std::env::set_current_dir(original_cwd).unwrap();
    }

    /// Test loading non-existent conversation history file
    #[test]
    fn test_conversation_history_load_nonexistent() {
        let temp_dir = create_temp_data_dir();
        let original_cwd = std::env::current_dir().unwrap();
        
        // Change to temp directory so "data" directory doesn't exist
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Try to load non-existent file - should return new empty history
        let result = ConversationHistory::load_from_file("nonexistent-session");
        assert!(result.is_ok());
        
        let history = result.unwrap();
        assert_eq!(history.session_id, "nonexistent-session");
        assert!(history.messages.is_empty());
        
        // Restore original directory
        std::env::set_current_dir(original_cwd).unwrap();
    }

    /// Test UUID generation and validation
    #[test]
    fn test_uuid_generation() {
        // Test that we can generate valid UUIDs
        let uuid1 = Uuid::new_v4().to_string();
        let uuid2 = Uuid::new_v4().to_string();
        
        // Should be valid UUIDs
        assert!(Uuid::parse_str(&uuid1).is_ok());
        assert!(Uuid::parse_str(&uuid2).is_ok());
        
        // Should be different
        assert_ne!(uuid1, uuid2);
    }

    /// Test UUID parsing validation
    #[test]
    fn test_uuid_parsing() {
        // Valid UUID should parse
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(Uuid::parse_str(valid_uuid).is_ok());
        
        // Invalid UUID should not parse
        let invalid_uuid = "invalid-uuid-string";
        assert!(Uuid::parse_str(invalid_uuid).is_err());
        
        // Empty string should not parse
        assert!(Uuid::parse_str("").is_err());
    }
} 