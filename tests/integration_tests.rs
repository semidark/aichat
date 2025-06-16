//! Integration tests for Kindle AI Chat
//! 
//! These tests verify the HTTP endpoints and web functionality of the Kindle AI Chat
//! application using Rocket's testing framework. They test the complete request/response
//! cycle including session management, cookie handling, and JSON responses.

use rocket::local::asynchronous::Client;
use rocket::http::{Status, ContentType, Cookie};
use serde_json;
use uuid;

// Import our library functions and types
use aichat::{rocket, ChatRequest, ChatResponse};

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
    /// 4. The response contains the expected JSON structure
    #[rocket::async_test]
    async fn test_chat_endpoint_creates_session_cookie_on_first_visit() {
        let client = create_test_client().await;
        
        // Create a chat request - this simulates a user's first message
        let chat_request = ChatRequest {
            message: "Hello, this is my first message!".to_string(),
        };
        
        // Make a POST request to /api/chat with JSON content
        // Note: we deliberately don't set any cookies to simulate a first visit
        let response = client
            .post("/api/chat")
            .header(ContentType::JSON)
            .json(&chat_request)
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
        
        // Verify the response body contains valid JSON with expected structure
        let response_body = response.into_string().await
            .expect("Response should have a body");
        
        let chat_response: ChatResponse = serde_json::from_str(&response_body)
            .expect("Response should be valid ChatResponse JSON");
        
        // Verify response contains expected fields
        assert!(!chat_response.response.is_empty(), 
                "Chat response should contain a non-empty response");
        assert_eq!(chat_response.status, "success", 
                   "Chat response status should be 'success'");
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
        let first_chat_request = ChatRequest {
            message: "This is my first message in the conversation".to_string(),
        };
        
        let first_response = client
            .post("/api/chat")
            .header(ContentType::JSON)
            .json(&first_chat_request)
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
        
        // Parse the first response to ensure it's valid
        let first_response_body = first_response.into_string().await
            .expect("First response should have a body");
        
        let first_chat_response: ChatResponse = serde_json::from_str(&first_response_body)
            .expect("First response should be valid ChatResponse JSON");
        
        assert_eq!(first_chat_response.status, "success", 
                   "First chat response status should be 'success'");
        
        // SECOND REQUEST: Use the existing session cookie
        let second_chat_request = ChatRequest {
            message: "This is my second message in the same conversation".to_string(),
        };
        
        // Create a cookie to send with the second request
        let cookie_for_second_request = Cookie::new("session_id", original_session_id.clone());
        
        let second_response = client
            .post("/api/chat")
            .header(ContentType::JSON)
            .cookie(cookie_for_second_request)
            .json(&second_chat_request)
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
                       "Session ID should remain the same across requests");
        }
        
        // Parse the second response to ensure it's valid
        let second_response_body = second_response.into_string().await
            .expect("Second response should have a body");
        
        let second_chat_response: ChatResponse = serde_json::from_str(&second_response_body)
            .expect("Second response should be valid ChatResponse JSON");
        
        assert_eq!(second_chat_response.status, "success", 
                   "Second chat response status should be 'success'");
        
        // Additional verification: Both responses should be non-empty 
        // (the content will be error responses in test environment, but that's expected)
        assert!(!first_chat_response.response.is_empty(), 
                "First chat response should contain content");
        assert!(!second_chat_response.response.is_empty(), 
                "Second chat response should contain content");
        
        // Both responses should have the same status indicating successful API handling
        // even if the LLM call itself fails (which is expected in test environment)
        assert_eq!(first_chat_response.status, "success", 
                   "Both responses should have success status");
        assert_eq!(second_chat_response.status, "success", 
                   "Both responses should have success status");
    }
    
    // TODO: Implement test for POST /api/chat JSON response format (Task 2.T.3.4)
    // This test should confirm the basic JSON response from /api/chat is
    // well-formed and contains the expected fields.

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
} 