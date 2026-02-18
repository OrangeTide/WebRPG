# WebRPG

## Introductions

WebRPG is a virtual tabletop roleplaying game system.

This server hosts roleplaying sessions. A session is a suite of tools useful
for roleplaying, including: dice rolling, maps with fog of war, character
sheets, chat dialogs, note taking, party inventory, reference material,
resource tracking, spell books, combat encounters, initiative order, and turn
tracking.

## Developer Information

Server is built with Rust + leptos.

Clients connect with a browser and receieve a WASM payload developed with leptos.

### Build Prerequisites

Install wasm32 Rust support:
  ```
  rustup target add wasm32-unknown-unknown
  ```

### Testing Requirements

  * [Firefox 115.32.0 ESR](https://ftp.mozilla.org/pub/firefox/releases/115.32.0esr/) - needed for testing of Windows 7-8.1 and macOS 10.12-10.14
  * Install this in a temporary directory or container. (tested as working on Zorin OS 17)
