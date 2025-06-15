# Product Requirements Document: Kindle AI Chat

## 1. Introduction / Overview
The goal is to develop a simple yet powerful AI-chat web interface optimized for low-power devices—specifically, the Kindle e-reader’s web browser.  
The application will feature:

* A Rust backend using the Rocket framework.
* A hypermedia-driven frontend powered by htmx.
* AI inference via the existing `@client` crate from aichat.

The first release focuses on a lean, fast, and responsive chat experience for resource-constrained clients with e-ink displays. Later iterations will add document-chat (RAG) capabilities via aichat’s `@rag` implementation.

---

## 2. Goals
* **Performance** – Extremely lightweight interface that feels snappy on the Kindle browser.  
* **Reusability** – Re-use the `@client` crate for all LLM communication.  
* **User Experience (UX)** – Smooth, readable streaming that respects slow e-ink refresh rates.  
* **Extensibility** – A solid foundation that can later add RAG, model-selection, etc.  
* **Developer Velocity** – An in-app debug console for fast testing directly on device.

---

## 3. User Stories
* **Core Interaction** – As a Kindle user, I can open the page and start chatting immediately—no login or setup.  
* **Conversation Continuity** – My session history is remembered so I can ask follow-up questions at any time.  
* **E-Ink-Optimized Streaming** – Replies appear in smooth, reasonably sized chunks rather than flashing character-by-character.  
* **Developer Debugging** – I can toggle a debug console to view server and client logs on the Kindle.  
* **Simplified Workflow** – The UI is clean and minimal, letting me focus entirely on the conversation.

---

## 4. Functional Requirements

### Backend (Rocket Server)
1. **Serve Interface** – Serve a single static `index.html` file containing the entire UI.  
2. **Chat Endpoint** – Expose `/chat` (POST) for user messages from htmx.  
3. **LLM Integration** – Use the `@client` crate for all model calls (initially hard-coded model).  
4. **Persistent Session Management** –  
   * On first visit, generate a UUID and set it as a persistent cookie.  
   * Store each user’s conversation history on disk, keyed by this UUID, enabling session continuity across visits.  
5. **Configurable Streaming** –  
   * Stream AI responses in chunks.  
   * Chunk size and inter-chunk delay are configurable via `Rocket.toml`; environment variables can override these values for quick tuning on target devices.

### Frontend (htmx on Kindle Browser)
1. **Minimalist UI**  
   * Conversation history display area (plain text for v1; markdown formatting later).  
   * Resizable `<textarea>` for user input.  
   * Submit button.  
2. **htmx-Powered Interactions** – All actions (e.g., submit) trigger htmx POST requests and HTML fragment swaps.  
3. **Streamed Updates** – Append streamed chunks to the conversation display in real time.  
4. **Debug Log**  
   * Toggleable console pane.  
   * Shows server log lines and client events for on-device debugging.  
5. **Kindle Compatibility** – Strict ES5-only JS, simple CSS, and layouts compatible with Kindle firmware 5.16.4+.

---

## 5. Non-Goals (MVP)
* User accounts / authentication.  
* Multiple simultaneous chat tabs.  
* Model selection UI.  
* Editing/deleting messages or branching conversations.  
* RAG (document chat).

---

## 6. Design & Technical Considerations
* **UI Design** – High-contrast, text-focused layout with large, readable fonts for e-ink.  
* **Framework Choices** – Rocket (backend) and htmx (frontend) for minimal client JS.  
* **JavaScript** – ES5-compliant only; Kindle browser lacks modern ES6 features.  
* **Styling** – Tiny CSS footprint; avoid flexbox/grid in favor of block/inline-block.  
* **Configuration** – `Rocket.toml` (with env-var overrides) controls streaming chunk size and delay, enabling live tuning for optimal e-ink performance.

---

## 7. Success Metrics
* App loads and becomes interactive within 2 s on Kindle Paperwhite 11th Gen (FW 5.16.4+) over stable Wi-Fi.  
* Users complete multi-turn conversations with correct context retention.  
* Streaming feels smooth, with no excessive flicker on e-ink.  
* In-app debug console reliably displays log output.
