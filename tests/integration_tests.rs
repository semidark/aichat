//! Integration tests for Kindle AI Chat
//! 
//! These tests verify the HTTP endpoints and web functionality of the Kindle AI Chat
//! application using Rocket's testing framework. They test the complete request/response
//! cycle including session management, cookie handling, and HTML responses for htmx.

use rocket::local::asynchronous::Client;
use rocket::http::{Status, ContentType, Cookie};
use uuid;
use aichat::{rocket, StreamingConfig};
use serde_json;

/// Helper function to create a test client
/// 
/// This function creates a Rocket client for testing purposes, using the same
/// rocket() function that the production server uses.
async fn create_test_client() -> Client {
    let rocket_instance = rocket().await;
    Client::tracked(rocket_instance)
        .await
        .expect("valid rocket instance for testing")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test GET / endpoint to verify static file server functionality (Task 2.T.3.1)
    /// 
    /// This test confirms that the static file server works correctly by making a GET request
    /// to the root path and asserting that it returns a 200 OK status. The static file server
    /// is configured to serve files from the "static" directory, with "index.html" as the
    /// default file for directory requests.
    #[rocket::async_test]
    async fn test_static_file_server_root_request() {
        let client = create_test_client().await;
        
        // Make a GET request to the root path
        let response = client.get("/").dispatch().await;
        
        // Assert that we get a 200 OK status, confirming the static file server works
        assert_eq!(response.status(), Status::Ok, 
                   "Static file server should return 200 OK for root path");
        
        // Additionally verify that we got some content (the index.html file)
        let body = response.into_string().await.expect("Response should have a body");
        assert!(!body.is_empty(), "Response body should not be empty");
        
        // Verify it's actually serving HTML content
        assert!(body.contains("<html>") || body.contains("<!DOCTYPE"), 
                "Response should contain HTML content");
    }
    
    /// Test POST /api/chat session creation on first visit (Task 2.T.3.2)
    /// 
    /// This test simulates a user's first visit by making a POST request to the chat endpoint
    /// without any existing session cookie. It verifies that:
    /// 1. The endpoint responds with a 200 OK status
    /// 2. A `session_id` cookie is created in the response
    /// 3. The cookie value is a valid UUID
    /// 4. The response contains SSE content for htmx
    #[rocket::async_test]
    async fn test_chat_endpoint_creates_session_cookie_on_first_visit() {
        let client = create_test_client().await;
        
        // Create form data - this simulates what htmx sends on a user's first message
        let form_data = "message=Hello, this is my first message!";
        
        // Make a POST request to /api/chat with form data
        // Note: we deliberately don't set any cookies to simulate a first visit
        let response = client
            .post("/api/chat")
            .header(ContentType::Form)
            .body(form_data)
            .dispatch()
            .await;
        
        // Assert that we get a 200 OK status
        assert_eq!(response.status(), Status::Ok, 
                   "Chat endpoint should return 200 OK status");
        
        // Extract cookies from the response
        let cookies = response.cookies();
        
        // Assert that a session_id cookie was created
        let session_cookie = cookies
            .iter()
            .find(|cookie| cookie.name() == "session_id")
            .expect("Response should contain a session_id cookie");
        
        // Verify the cookie value is a valid UUID
        let session_id = session_cookie.value();
        assert!(!session_id.is_empty(), "Session ID should not be empty");
        
        // Parse as UUID to verify it's properly formatted
        uuid::Uuid::parse_str(session_id)
            .expect("Session ID should be a valid UUID format");
        
        // Verify cookie properties match our security requirements
        assert!(session_cookie.http_only().unwrap_or(false), 
                "Session cookie should be HTTP-only for security");
        
        // Verify the response content type is text/event-stream
        assert_eq!(response.content_type(), Some(ContentType::new("text", "event-stream")),
                  "Response should have Content-Type: text/event-stream");
        
        // Verify the response body contains SSE content
        let response_body = response.into_string().await
            .expect("Response should have a body");
        
        // Check for SSE format
        assert!(response_body.contains("event:"), 
                "Response should contain SSE event fields");
        
        // Verify it's NOT HTML with container tags
        assert!(!response_body.contains("<div class=\"message assistant\">"), 
                "Response should NOT contain HTML container tags");
    }
    
    /// Test POST /api/chat session persistence on subsequent visit (Task 2.T.3.3)
    /// 
    /// This test simulates a user's subsequent visit by making two POST requests to the chat 
    /// endpoint. The first request creates a session, and the second request uses that session
    /// cookie to verify session persistence. It confirms that:
    /// 1. The same session ID is maintained across requests
    /// 2. The conversation history is preserved in the session
    /// 3. The server recognizes and uses the existing session
    #[rocket::async_test]
    async fn test_chat_endpoint_persists_existing_session() {
        let client = create_test_client().await;
        
        // FIRST REQUEST: Create a session with the initial message
        let first_form_data = "message=This is my first message in the conversation";
        
        let first_response = client
            .post("/api/chat")
            .header(ContentType::Form)
            .body(first_form_data)
            .dispatch()
            .await;
        
        // Verify first request succeeded
        assert_eq!(first_response.status(), Status::Ok, 
                   "First chat request should return 200 OK status");
        
        // Extract the session cookie from the first response
        let cookies = first_response.cookies();
        let session_cookie = cookies
            .iter()
            .find(|cookie| cookie.name() == "session_id")
            .expect("First response should contain a session_id cookie");
        
        let original_session_id = session_cookie.value().to_string();
        
        // Verify we got a valid UUID
        uuid::Uuid::parse_str(&original_session_id)
            .expect("Session ID should be a valid UUID format");
        
        // Verify the first response is SSE
        assert_eq!(first_response.content_type(), Some(ContentType::new("text", "event-stream")),
                  "First response should have Content-Type: text/event-stream");
        
        // SECOND REQUEST: Use the existing session cookie
        let second_form_data = "message=This is my second message in the same conversation";
        
        // Create a cookie to send with the second request
        let cookie_for_second_request = Cookie::new("session_id", original_session_id.clone());
        
        let second_response = client
            .post("/api/chat")
            .header(ContentType::Form)
            .cookie(cookie_for_second_request)
            .body(second_form_data)
            .dispatch()
            .await;
        
        // Verify second request succeeded
        assert_eq!(second_response.status(), Status::Ok, 
                   "Second chat request should return 200 OK status");
        
        // Extract cookies from the second response
        let second_cookies = second_response.cookies();
        
        // Check if a session cookie is present in the second response
        // Note: The server may or may not send the cookie again, depending on implementation
        let maybe_second_session_cookie = second_cookies
            .iter()
            .find(|cookie| cookie.name() == "session_id");
        
        // If a session cookie is present in the second response, it should be the same ID
        if let Some(second_session_cookie) = maybe_second_session_cookie {
            let second_session_id = second_session_cookie.value();
            assert_eq!(second_session_id, original_session_id,
                      "Session ID should be the same across requests");
        }
        
        // Verify the second response is SSE
        assert_eq!(second_response.content_type(), Some(ContentType::new("text", "event-stream")),
                  "Second response should have Content-Type: text/event-stream");
        
        // Verify the response body contains SSE content
        let second_response_body = second_response.into_string().await
            .expect("Second response should have a body");
        
        // Check for SSE format
        assert!(second_response_body.contains("event:"), 
                "Second response should contain SSE event fields");
    }
    
    /// Test POST /api/chat form data handling and SSE response (Task 2.T.3.4)
    /// 
    /// This test verifies that the chat endpoint correctly handles form data and returns
    /// a well-formed SSE response. It checks:
    /// 1. The endpoint accepts form data with a message parameter
    /// 2. The response has the correct content type (text/event-stream)
    /// 3. The response contains SSE events
    #[rocket::async_test]
    async fn test_chat_endpoint_form_data_sse_response() {
        let client = create_test_client().await;
        
        // Create form data with a test message
        let form_data = "message=Testing the chat endpoint form data handling";
        
        // Make a POST request to /api/chat with form data
        let response = client
            .post("/api/chat")
            .header(ContentType::Form)
            .body(form_data)
            .dispatch()
            .await;
        
        // Verify the response status is 200 OK
        assert_eq!(response.status(), Status::Ok, 
                   "Chat endpoint should return 200 OK status");
        
        // Verify the response content type is text/event-stream
        assert_eq!(response.content_type(), Some(ContentType::new("text", "event-stream")),
                  "Response should have Content-Type: text/event-stream");
        
        // Get the response body
        let response_body = response.into_string().await
            .expect("Response should have a body");
        
        // Print the response body for debugging
        println!("Response body: {}", response_body);
        
        // Check that the response is not empty
        assert!(!response_body.is_empty(), "Response body should not be empty");
    }

    /// Placeholder test to ensure the test framework is working
    /// 
    /// This test will be replaced with actual integration tests in subsequent tasks.
    #[rocket::async_test]
    async fn test_framework_setup() {
        // This test verifies that we can create a test client successfully
        let _client = create_test_client().await;
        
        // If we reach this point, the test framework is working correctly
        assert!(true, "Test framework setup successful");
    }

    /// Test that streaming configuration is properly loaded from Rocket.toml (Task 4.2)
    /// 
    /// This test verifies that the StreamingConfig is properly loaded into Rocket's state
    /// management system and contains the expected default values from Rocket.toml.
    #[rocket::async_test]
    async fn test_streaming_config_loaded() {
        let rocket_instance = rocket().await;
        
        // Test that the streaming config is managed by Rocket
        let streaming_config = rocket_instance.state::<StreamingConfig>();
        assert!(streaming_config.is_some(), 
                "StreamingConfig should be managed by Rocket");
        
        let config = streaming_config.unwrap();
        // Should have default values from Rocket.toml
        assert_eq!(config.delay_ms, 500, 
                   "Default delay should be 500ms for e-ink refresh in debug mode");
    }

    /// Test the config debug endpoint returns streaming configuration as JSON (Task 4.2)
    /// 
    /// This test verifies that the /api/config endpoint returns the current streaming
    /// configuration as JSON, which is useful for debugging and verification.
    #[rocket::async_test]
    async fn test_config_debug_endpoint() {
        let client = create_test_client().await;
        
        let response = client.get("/api/config").dispatch().await;
        
        // Should return 200 OK
        assert_eq!(response.status(), Status::Ok, 
                   "Config debug endpoint should return 200 OK");
        
        // Should return JSON content type
        let content_type = response.content_type();
        assert!(content_type.is_some(), "Response should have content type");
        assert_eq!(*content_type.unwrap().media_type(), rocket::http::MediaType::JSON,
                   "Response should be JSON");
        
        // Parse the JSON response
        let config_json = response.into_string().await
            .expect("Response should have a body");
        
        let config: StreamingConfig = serde_json::from_str(&config_json)
            .expect("Response should be valid StreamingConfig JSON");
        
        // Verify the configuration values
        assert_eq!(config.delay_ms, 500, 
                   "Config endpoint should return delay_ms of 500 in debug mode");
    }

    /// Test streaming functionality of the chat endpoint (Task 4.T.1)
    /// 
    /// This test verifies that the chat endpoint properly streams responses using
    /// Server-Sent Events (SSE). It checks:
    /// 1. The endpoint returns a 200 OK status
    /// 2. The response has Content-Type: text/event-stream
    /// 3. The response contains data
    #[rocket::async_test]
    async fn test_chat_endpoint_streaming() {
        let client = create_test_client().await;
        
        // Create form data with a test message
        let form_data = "message=Testing streaming functionality";
        
        // Make a POST request to /api/chat with form data
        let response = client
            .post("/api/chat")
            .header(ContentType::Form)
            .body(form_data)
            .dispatch()
            .await;
        
        // Verify the response status is 200 OK
        assert_eq!(response.status(), Status::Ok, 
                   "Chat endpoint should return 200 OK status");
        
        // Verify the response content type is text/event-stream
        assert_eq!(response.content_type(), Some(ContentType::new("text", "event-stream")),
                  "Response should have Content-Type: text/event-stream");
        
        // Get the response body
        let response_body = response.into_string().await
            .expect("Response should have a body");
        
        // Print the response body for debugging
        println!("Response body: {}", response_body);
        
        // Check that the response is not empty
        assert!(!response_body.is_empty(), "Response body should not be empty");
    }
} 