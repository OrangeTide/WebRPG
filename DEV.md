# Developer Information

## Overview

Server is built with Rust + leptos.

Clients connect with a browser and receive a WASM payload developed with leptos.

Authentication is performed with JWT. This is handled using jsonwebtoken with chrono to handle JWT expiration.

TODO: describe what this server does.

## Project Structure

```
webrpg
├── src/
│   ├── auth.rs        # Authentication logic (hashing, JWT)
│   ├── models.rs      # User models and structs
│   ├── routes.rs      # API routes (login, signup, protected)
│   ├── main.rs        # Entry point and server setup
│   └── schema.rs      # Diesel's SQL schema mappings to Rust
└── Cargo.toml
```

## Build

### Build  Prerequisites

Install wasm32 Rust support:
  ```
  rustup target add wasm32-unknown-unknown
  ```

Install sqlite3 (e.g. on debian):
  ```
  sudo apt-get install sqlite3 sqlite3-tools libsqlite3-dev
  ```

## Components

### Configuration

Uses [Crate config](https://docs.rs/config/latest/config/) to load configuration from a settings file and from the environment.

### models.rs

In src/models.rs, define the user model and structs for API requests:

## Testing

### Supported Platforms

  * [Firefox 115.32.0 ESR](https://ftp.mozilla.org/pub/firefox/releases/115.32.0esr/)
    * Install this in a temporary directory or container. (tested as working on Zorin OS 17)
  * Chrome 138.0 (LTS-138) [Long Term Support](https://support.google.com/chrome/a/answer/11333726)

## Design Details

### Pages

Landing page - this page provides an introduction to the website, and information on the current status. It is publicly visible. A link to the login page is here.

Login page - Handles user authentication via JWT. It returns user back to the previous page after successful login.

Game session selection page - Lists sessions visible to the current user.

### Authentication

TBD


