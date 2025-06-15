# Product Requirements Document  
### Kindle AI Chat (aichat-kindle Edition)

## 1. Introduction / Overview
The goal is to develop a simple yet powerful AI-chat web interface optimized for low-power devices—specifically, the Kindle e-reader’s web browser.  


Kindle AI Chat is a fork-based adaptation of the open-source project **sigoden/aichat**, purpose-built for the low-power web browser found on Kindle e-readers (FW 5.16.4 +).  
By leveraging aichat’s well-tested crates and architecture, we will deliver a lightweight, snappy chat experience on e-ink while keeping long-term parity with upstream improvements.

Key characteristics
* Rust backend on Rocket (mirrors aichat’s Rust foundations).
* Hypermedia frontend driven by htmx (minimal ES5-only JavaScript).
* AI inference, session logic, and streaming powered by aichat’s `@client` and related crates.

The first release focuses on a lean, fast, and responsive chat experience for resource-constrained clients with e-ink displays. Later iterations will add document-chat (RAG) capabilities via aichat’s `@rag` implementation.

---
### 1-A. Code Reuse & Forking Strategy
1. **Project Fork** – Development begins with a fork of `github.com/sigoden/aichat` under the working name `aichat-kindle`.  
2. **Module Reuse**  
   • Reuse unchanged: `@client`, conversation history structs, logging utilities.  
   • Adapt: server bootstrap (Rocket instead of axum), HTML templates for Kindle constraints.  
3. **Upstream Compatibility** –  
   • Keep crate boundaries identical wherever possible.  
   • Submit generic improvements back to the upstream repo.  

---

## 2. Goals
* **Performance** – Extremely lightweight interface that feels snappy on the Kindle browser.  
* **Codebase Reusability** – Maximise reuse of aichat code to accelerate delivery and simplify future maintenance.  
* **User Experience (UX)** – Smooth, chunked streaming optimised for slow e-ink refresh.  
* **Extensibility** – The fork must stay structurally compatible with upcoming aichat features (e.g., `@rag`, model selection).  
* **Developer Velocity** – On-device debug console for rapid iteration.

---

## 3. User Stories
* **Core Interaction** – As a Kindle user, I can open the page and start chatting immediately—no login or setup.
* **Conversation Continuity** – My chat history persists across visits via cookie-keyed sessions.
* **E-Ink-Optimised Streaming** – Replies appear in smooth, reasonably sized chunks.
* **Developer Debugging** – I can toggle a console to view logs directly on Kindle.
* **Simplified Workflow** – The UI is clean and simple.

---

## 4. Functional Requirements

### 4.1 Backend (Rocket Server)
1. **Serve Interface** – Single `index.html` (forked from aichat’s template, Kindle-tuned).
2. **Chat Endpoint** – `POST /chat` receives user input and streams response.
3. **LLM Integration** – Use aichat’s `@client` crate (and any future shared crates) for model calls.
4. **Persistent Sessions**
   * Generate UUID cookie on first visit.
   * Store conversation JSON on disk under `data/{uuid}.json` (same schema as aichat).
5. **Configurable Streaming**
   * Chunk size & delay in `Rocket.toml` (env-var overridable).
   * Defaults tuned for Kindle (e.g., 24 chars, 300 ms).
6. **Logging & Debug Export** – Mirror aichat log format; expose `/logs?tail` SSE for client console.

### 4.2 Frontend (htmx on Kindle Browser)
1. **Minimalist UI**  
   * Scrollable history pane (plain text v1).
   * Resizable `<textarea>` + submit button.
2. **htmx Interactions** – Forms POST to `/chat`; response streamed and appended.
3. **Stream Handling** – JavaScript (ES5) concatenates server-sent `<span>` chunks to history.
4. **Debug Console** – Toggle pane fetches `/logs?tail` via EventSource.
5. **Kindle Compatibility** – ES5 only, no flexbox/grid, high-contrast CSS ≤ 5 KB.

---

## 5. Non-Goals (MVP)
* Authentication & multi-user accounts.
* Multiple chat tabs/branches.
* UI model selection. 
* Message editing/deletion.
* RAG (document chat).

---

## 6. Design & Technical Considerations
* **High-Contrast UI** – Large serif font (18 px), 1.4 line-height for e-ink clarity.
* **Framework Choices** – Rocket chosen for its ergonomic async streaming; aligns with Rust ecosystem and is easy to graft onto aichat logic.
* **JavaScript Constraints** – Kindle WebKit ≈ Safari 5: stick to ES5 and DOM-1 APIs.
* **Styling Footprint** – `<1 KB` critical CSS; no external fonts.
* **Configuration Management** – `Rocket.toml` with `KINDLE_*` env override.  
* **Upstream Sync** – Keep crate namespaces identical to aichat to minimise merge friction.

---

## 7. Success Metrics
* ≤ 2 s to interactive on Kindle Paperwhite 11th Gen.
* ≥ 95 % of streamed chunks arrive without visible Flash/Flicker artifacts.  
* Session history persists after browser restart (verified across 20 test runs).
* Debug console displays latest 100 log lines with < 200 ms lag.
