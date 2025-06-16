//! Integration tests for Kindle AI Chat
//! 
//! These tests verify the HTTP endpoints and web functionality of the Kindle AI Chat
//! application using Rocket's testing framework. They test the complete request/response
//! cycle including session management, cookie handling, and JSON responses.

use rocket::local::asynchronous::Client;
use rocket::http::{Status, ContentType};
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
    
    // TODO: Implement test for POST /api/chat session persistence (Task 2.T.3.3)
    // This test should simulate a subsequent visit by sending a cookie and
    // verify the server uses the existing session.
    
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