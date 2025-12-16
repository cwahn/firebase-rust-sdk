# Firebase Rust SDK

[![Crates.io](https://img.shields.io/crates/v/firebase-rust-sdk.svg)](https://crates.io/crates/firebase-rust-sdk)
[![Documentation](https://docs.rs/firebase-rust-sdk/badge.svg)](https://docs.rs/firebase-rust-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Idiomatic Rust SDK for Firebase Authentication and Cloud Firestore with full async/await support and gRPC transport.

> **⚠️ Unofficial Port**: This is an unofficial community port of the Firebase C++ SDK. It is not affiliated with, endorsed by, or supported by Google or Firebase.
>
> **⚠️ Alpha Release**: This is version 0.1.0-alpha.1. APIs may change before 1.0.0. Use at your own risk in production.

## Features

### Firebase Authentication
- ✅ Email/password authentication
- ✅ Anonymous authentication  
- ✅ OAuth providers (Google, Facebook, GitHub, generic OAuth)
- ✅ Custom token authentication
- ✅ Password reset
- ✅ Automatic token refresh
- ✅ User management (profile, password, email updates)
- ✅ Auth state change listeners (Rust Streams)

### Cloud Firestore
- ✅ Document CRUD operations via gRPC
- ✅ Queries with filters, ordering, pagination
- ✅ WriteBatch for atomic operations
- ✅ Transactions with automatic retry
- ✅ Real-time listeners using gRPC streaming
- ✅ Nested collections
- ✅ Compound filters (And/Or)
- ✅ GeoPoint, Timestamp support
- ⚠️ Offline persistence API (structure ready, implementation pending)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
firebase-rust-sdk = "0.1.0-alpha.1"
tokio = { version = "1", features = ["full"] }
```

## Quick Start

See the [documentation](https://docs.rs/firebase-rust-sdk) for detailed API reference.

### Setup

1. Create a Firebase project at [console.firebase.google.com](https://console.firebase.google.com)
2. Enable Authentication and Firestore in your project
3. Get your API key and project ID from project settings

### Basic Usage

```rust
use firebase_rust_sdk::{App, AppOptions, Auth, firestore::Firestore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Firebase App
    let app = App::create(AppOptions {
        api_key: "YOUR_API_KEY".to_string(),
        project_id: "your-project-id".to_string(),
        app_name: None,
    }).await?;
    
    // Get Auth instance
    let auth = Auth::get_auth(&app).await?;
    
    // Sign in
    auth.sign_in_with_email_and_password("user@example.com", "password").await?;
    let user = auth.current_user().await?;
    println!("Signed in as: {}", user.uid);
    
    // Get Firestore instance with ID token
    let id_token = user.get_id_token(false).await?;
    let firestore = Firestore::new(
        app.options.project_id.clone(),
        "default".to_string(),
        Some(id_token)
    ).await?;
    
    // Write document
    let doc_ref = firestore.document("users/alice");
    doc_ref.set(/* your data */).await?;
    
    // Read document
    let snapshot = doc_ref.get().await?;
    if snapshot.exists() {
        println!("Data: {:?}", snapshot.data);
    }
    
    Ok(())
}
```

## Requirements

- Rust 1.75 or later
- Tokio runtime
- Firebase project with Authentication and Firestore enabled

## Architecture

- **Auth**: REST API for authentication operations
- **Firestore**: gRPC with TLS for all database operations
- **Async**: Full async/await with Tokio
- **Streaming**: Real-time listeners use Rust Stream trait
- **Type-safe**: Strong typing for all Firebase types

## Development Status

This SDK is in active development. Core functionality is complete, but some features are pending:

- ✅ All Auth features implemented and tested
- ✅ All Firestore CRUD operations working
- ✅ Real-time listeners with gRPC streaming
- ✅ Transactions and batched writes
- ⚠️ Offline persistence (API ready, implementation pending)

## Testing

The SDK includes 24 comprehensive integration tests. To run them:

1. Copy `.env.example` to `.env`
2. Fill in your Firebase credentials
3. Run: `cargo test --test firestore_integration -- --test-threads=1`

## Contributing

Contributions welcome! This project follows the design patterns from the Firebase C++ SDK while using idiomatic Rust patterns.

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Credits

Based on the Firebase C++ SDK architecture. This is an independent implementation and is not officially supported by Google or Firebase.
