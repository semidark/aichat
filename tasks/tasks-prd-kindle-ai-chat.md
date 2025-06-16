# Tasks for the Kindle AI Chat project

Based on PRD: `prd-kindle-ai-chat.md`

## Relevant Files

- `src/main.rs` - The main entry point for the Rocket web server application.
- `src/main_original_backup.rs` - Backup of the original aichat CLI main.rs for reference.
- `static/index.html` - The single HTML file for the user interface, including htmx attributes.
- `static/style.css` - Minimal, high-contrast CSS optimized for Kindle's e-ink display.
- `static/htmx.min.js` - The htmx library, served locally.
- `static/client.js` - Minimal ES5-compliant JavaScript for handling streamed responses and the debug console.
- `Rocket.toml` - Configuration file for the Rocket server, including streaming parameters.
- `data/` - Directory to store persistent conversation history as JSON files.
- `tests/integration_tests.rs` - Integration tests for the Rocket endpoints.
- `scripts/test-curl.sh` - Automated curl test suite for manual verification of HTTP endpoints.

### Notes

- Unit tests should be placed alongside the code files they are testing.
- Use `cargo test` to run all tests.

## Tasks

- [x] **0 Fork aichat and create a branch for the kindle-ai-chat project**
- [x] **1.0 Project Setup and Initial Rocket Integration**
  - [x] 1.1 Add `rocket` (0.5.1) and `uuid` (1.17.0) to `Cargo.toml`.
  - [x] 1.2 Replace the existing `aichat` CLI entry point with a Rocket server launch in `src/main.rs`.
  - [x] 1.3 Create a `static/` directory to serve static assets (`index.html`, `css`, `js`).
  - [x] 1.4 Implement a Rocket route to serve files from the `static/` directory.
  - [x] 1.5 Create a basic `static/index.html` with a "Hello World" message to confirm the server is working.

- [ ] **2.0 Implement Backend Core Chat and Session Logic**
  - [x] 2.1 Create a `POST /chat` endpoint in Rocket to receive user input.
  - [x] 2.2 Implement cookie-based session handling: on first visit, generate a UUID, set it as a persistent cookie, and create a `data/{uuid}.json` file.
  - [x] 2.3 On subsequent requests, read the UUID from the cookie to load the corresponding conversation history from the JSON file.
  - [x] 2.4 Integrate `aichat`'s `@client` crate to send the user's prompt (with history) to the LLM.
  - [ ] 2.5 For now, have the `/chat` endpoint return the entire AI response in a single block, updating the session file.

  - [ ] **2.T Retroactive Testing (Covering Tasks 1.0-2.4)**
  - [ ] **2.T.1 Refactor for Testability & Create Integration Test Harness**
    - [x] 2.T.1.1 Move Rocket instance creation from `run_server()` into a new public `rocket()` function in `src/main.rs` so it can be imported by tests.
    - [x] 2.T.1.2 Update `run_server()` to call the new `pub fn rocket()` function.
    - [x] 2.T.1.3 Create a `src/lib.rs` and move the application logic there, turning our binary into a library that integration tests can use. `src/main.rs` will now just call the library.
    - [x] 2.T.1.4 Create the `tests/` directory and an empty `tests/integration_tests.rs` file.

  - [ ] **2.T.2 Implement Unit Tests for Core Session Logic**
    - [ ] 2.T.2.1 Add a `#[cfg(test)]` module at the bottom of the file containing the core logic (`src/lib.rs` after refactor).
    - [ ] 2.T.2.2 Write a unit test for `ConversationHistory` to verify saving to and loading from a temporary file works correctly.
    - [ ] 2.T.2.3 Write a unit test for `to_conversation_text()` to ensure it formats the prompt history correctly for the LLM.
    - [ ] 2.T.2.4 Write unit tests for `get_or_create_session_id()` to validate both the creation of a new session cookie and the retrieval of an existing one.

  - [ ] **2.T.3 Implement Integration Tests for Web Endpoints**
    - [ ] 2.T.3.1 In `tests/integration_tests.rs`, write a test to make a `GET /` request and assert a `200 OK` status to confirm the static file server works.
    - [ ] 2.T.3.2 Write an integration test for `POST /api/chat` that simulates a user's first visit and asserts that a `session_id` cookie is successfully created in the response.
    - [ ] 2.T.3.3 Write an integration test that simulates a subsequent visit by sending a cookie and verifies the server uses the existing session.
    - [ ] 2.T.3.4 Write an integration test to confirm the basic JSON response from `/api/chat` is well-formed.

- [ ] **3.0 Build the Frontend UI with htmx**
  - [ ] 3.1 Download `htmx.min.js` (1.9.12) and place it in the `static/` directory.
  - [ ] 3.2 Structure `static/index.html` with a scrollable history pane, a resizable `<textarea>`, and a submit button.
  - [ ] 3.3 Use htmx attributes (`hx-post`, `hx-target`, `hx-swap`) on the form to send data to `/chat` and append the response to the history pane.
  - [ ] 3.4 Create `static/style.css` with a minimal, high-contrast, single-column layout using a large serif font suitable for e-ink.

- [ ] **4.0 Develop E-Ink Optimized Streaming**
  - [ ] 4.1 Modify the `/chat` endpoint to return a `Stream` of data.
  - [ ] 4.2 Add `streaming.chunk_size` and `streaming.delay_ms` to `Rocket.toml` and read them into the application's configuration.
  - [ ] 4.3 In the stream, send back small HTML fragments (e.g., `<span>chunk</span>`) for each chunk of the AI response, respecting the configured size and delay.
  - [ ] 4.4 Write a small, ES5-compatible function in `static/client.js` to handle htmx's streaming events and append the `<span>` fragments to the conversation display.

- [ ] **5.0 Implement the On-Device Debug Console**
  - [ ] 5.1 Create a new Rocket endpoint at `GET /logs/sse` that returns a Server-Sent Events (SSE) stream.
  - [ ] 5.2 Integrate this endpoint with `aichat`'s logging system to push formatted log messages to the SSE stream.
  - [ ] 5.3 Add a toggle button and a hidden `<div>` to `index.html` for the debug console.
  - [ ] 5.4 In `static/client.js`, write an ES5-compatible function that uses the `EventSource` API to connect to `/logs/sse` and display the received messages in the debug console `<div>`. 