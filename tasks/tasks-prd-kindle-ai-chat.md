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

### Notes

- Unit tests should be placed alongside the code files they are testing.
- Use `cargo test` to run all tests.

## Tasks

- [X] **0 Fork aichat and create a branch for the kindle-ai-chat project**
- [x] **1.0 Project Setup and Initial Rocket Integration**
  - [x] 1.1 Add `rocket` (0.5.1) and `uuid` (1.17.0) to `Cargo.toml`.
  - [x] 1.2 Replace the existing `aichat` CLI entry point with a Rocket server launch in `src/main.rs`.
  - [x] 1.3 Create a `static/` directory to serve static assets (`index.html`, `css`, `js`).
  - [x] 1.4 Implement a Rocket route to serve files from the `static/` directory.
  - [x] 1.5 Create a basic `static/index.html` with a "Hello World" message to confirm the server is working.

- [ ] **2.0 Implement Backend Core Chat and Session Logic**
  - [ ] 2.1 Create a `POST /chat` endpoint in Rocket to receive user input.
  - [ ] 2.2 Implement cookie-based session handling: on first visit, generate a UUID, set it as a persistent cookie, and create a `data/{uuid}.json` file.
  - [ ] 2.3 On subsequent requests, read the UUID from the cookie to load the corresponding conversation history from the JSON file.
  - [ ] 2.4 Integrate `aichat`'s `@client` crate to send the user's prompt (with history) to the LLM.
  - [ ] 2.5 For now, have the `/chat` endpoint return the entire AI response in a single block, updating the session file.

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