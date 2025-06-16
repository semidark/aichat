# Testing Onboarding Guide - Kindle AI Chat

## Quick Start for New Developers

## 🚀 Setup

1. **Run tests to verify your setup:**
   ```bash
   cargo test
   ```

2. **Enable helpful tools:**
   ```bash
   # Install cargo-watch for continuous testing
   cargo install cargo-watch
   
   # Run tests on file changes
   cargo watch -x test
   ```

## 🔧 Essential Testing Patterns

### 1. Basic Unit Test Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_your_function() {
        // Arrange
        let input = "test data";
        
        // Act
        let result = your_function(input);
        
        // Assert
        assert_eq!(result, expected_value);
    }
}
```

### 2. Async Testing (For Streaming/Web Features)
```rust
#[tokio::test]
async fn test_async_function() {
    let result = your_async_function().await;
    assert!(result.is_ok());
}
```

### 3. Testing Streams (Kindle E-ink Optimization)
```rust
// Use our custom macro for testing chunked streaming
#[tokio::test]
async fn test_streaming() {
    assert_json_stream!(input_data, expected_output);
}

### 4. Integration Testing (For Web Endpoints)
For testing our Rocket server's HTTP endpoints, we use integration tests. These live in the `tests/` directory at the root of our project (e.g., `tests/integration_tests.rs`).

They allow us to simulate real HTTP requests and verify endpoint behavior, session handling, and cookie management.

```rust
// In tests/integration_tests.rs
use rocket::local::blocking::Client;

#[test]
fn test_chat_endpoint_session() {
    // Build the Rocket instance for testing
    let rocket_instance = kindle_aichat::rocket(); // Assuming a function that returns the configured Rocket instance
    let client = Client::tracked(rocket_instance).expect("valid rocket instance");

    // Dispatch a request to the chat endpoint
    let response = client.post("/api/chat")
        .body(r#"{"message": "hello"}"#)
        .dispatch();
    
    // Assert status is OK
    assert_eq!(response.status(), rocket::http::Status::Ok);

    // Assert a session cookie was created
    assert!(response.cookies().get("session_id").is_some());
}
```

## 🎯 What to Test

### Priority 1: Core Functionality
- **AI Chat Logic**: Message processing, streaming responses
- **Session Management**: Cookie handling, conversation persistence
- **Streaming**: Chunk sizes and timing for e-ink displays

### Priority 2: Kindle-Specific Features
- **Performance**: Response times under 2s
- **Memory Usage**: Low footprint operations
- **E-ink Updates**: Batched updates, proper refresh timing

### Priority 3: Web Integration
- **Rocket Routes**: HTTP endpoints and responses
- **htmx Integration**: Streaming HTML updates
- **Error Handling**: Graceful degradation

## 🧪 Common Test Scenarios

### Testing a New Chat Function
```rust
#[test]
fn test_process_message() {
    let message = "Hello, AI!";
    let result = process_message(message);
    
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}
```

### Testing Streaming Response
```rust
#[tokio::test]
async fn test_chunked_response() {
    let stream = create_test_stream();
    let chunks = collect_chunks(stream).await;
    
    // Verify chunk sizes are appropriate for Kindle
    for chunk in chunks {
        assert!(chunk.len() <= 24); // Max chars per chunk
    }
}
```

### Testing Session Persistence
```rust
#[test]
fn test_session_storage() {
    let session_id = create_session();
    let message = "Test message";
    
    store_message(session_id, message);
    let retrieved = get_conversation(session_id);
    
    assert!(retrieved.contains(message));
}
```

## 🏃‍♂️ Running Tests

### Basic Commands
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_function_name

# Run tests with output
cargo test -- --nocapture

# Run tests in watch mode
cargo watch -x test
```

### Pre-commit Checks
```bash
# Full CI check (run before pushing)
cargo test && cargo clippy --all --all-targets -- -D warnings && cargo fmt --all --check
```

## 🚨 Common Gotchas

### 1. Async Tests
- Always use `#[tokio::test]` for async functions
- Don't forget `.await` on async calls
- Handle `Result` types properly

### 2. Stream Testing
- Use our `split_chunks()` helper for realistic chunk testing
- Test both success and error paths in streaming
- Verify timing constraints for e-ink displays

### 3. Cross-platform Testing
- Some tests have `#[cfg(not(target_os = "windows"))]`
- Check if your test needs OS-specific handling
- Use `PathBuf` for file paths, not string concatenation

## 🎪 Testing Utilities

### Custom Macros
We have helpful macros that reduce boilerplate:

```rust
// For JSON streaming tests
assert_json_stream!(input, expected_output);

// For template rendering tests
assert_render!(template, [(key, value)], expected);
```

### Helper Functions
```rust
// Create random chunks for stress testing
let chunks = split_chunks("your test data");

// Build test metadata
let metadata = build_metadata("test_id");
```

## 🔍 Debugging Tests

### Enable Detailed Output
```bash
# See println! output from tests
cargo test -- --nocapture

# Run specific failing test
cargo test failing_test_name -- --nocapture
```

### Use Debug Prints
```rust
#[test]
fn debug_test() {
    let result = your_function();
    println!("Debug: {:?}", result);  // Will show with --nocapture
    assert_eq!(result, expected);
}
```

**Remember**: Good tests make the codebase more reliable, especially important for Kindle's constrained environment. When in doubt, write a test! 🧪 

## 🌐 Manual Testing with Curl

While formal tests catch regressions, curl testing verifies real-world behavior:

### Automated Curl Test Suite

We provide a comprehensive test script that runs all curl tests automatically:

```bash
# Run the complete curl test suite
./scripts/test-curl.sh
```

This script will:
- ✅ Start the server automatically
- ✅ Run all HTTP endpoint tests
- ✅ Verify session management and cookies
- ✅ Test Kindle-specific headers
- ✅ Check session file persistence
- ✅ Provide a colorized summary report
- ✅ Clean up the server when finished

### Quick Curl Tests
```bash
# Test basic endpoint
curl http://localhost:8000/

# Test chat with streaming
curl -N http://localhost:8000/chat \
  -d '{"message": "Hello"}' \
  -H "Content-Type: application/json"

# Test with Kindle headers
curl -H "User-Agent: Kindle/3.0+" http://localhost:8000/
```

### When to Use Curl
- ✅ Debugging streaming issues
- ✅ Verifying performance (< 2s responses)  
- ✅ Testing real HTTP headers
- ✅ Manual exploration of new features
- ❌ Regression testing (use formal tests)
- ❌ Logic validation (use unit tests)

**Remember**: Keep both approaches:
- **Formal tests** = Your safety net and development speed
- **Curl testing** = Your verification tool for real-world behavior

For Kindle AI Chat specifically, curl testing is especially important because the e-ink constraints and streaming requirements mean you need to verify the **actual HTTP behavior**, not just the logic. 