# Getting Started with Firebase Rust SDK

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
firebase-rust-sdk = "0.1.0"
tokio = { version = "1.40", features = ["full"] }
serde_json = "1.0"
```

## Quick Start

### Authentication Example

```rust
use firebase_rust_sdk::Auth;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get auth instance
    let auth = Auth::get_auth("YOUR_API_KEY").await?;
    
    // Sign in with email/password
    let user = auth.sign_in_with_email_and_password(
        "user@example.com",
        "password"
    ).await?;
    
    println!("Signed in as: {} ({})", user.email, user.uid);
    Ok(())
}
```

### Firestore Example

```rust
use firebase_rust_sdk::firestore::Firestore;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let firestore = Firestore::get_firestore("my-project").await?;
    
    // Create document
    firestore.set_document(
        "users/alice",
        json!({
            "name": "Alice",
            "age": 30,
            "email": "alice@example.com"
        })
    ).await?;
    
    // Read document
    let doc = firestore.get_document("users/alice").await?;
    println!("Document: {:?}", doc.data);
    
    Ok(())
}
```

## Core Concepts

### Authentication

The Auth module provides user authentication through multiple providers:
- **Email/Password**: Traditional email-based accounts
- **Anonymous**: Guest/temporary accounts
- **OAuth**: Google, Facebook, GitHub, and custom providers
- **Custom Tokens**: Server-generated JWT tokens

#### Singleton Pattern

Auth instances are cached per API key:

```rust
// First call creates instance
let auth1 = Auth::get_auth("key1").await?;

// Subsequent calls return cached instance
let auth2 = Auth::get_auth("key1").await?; // Same instance as auth1
```

#### Token Management

Tokens are automatically refreshed:

```rust
let user = auth.sign_in_with_email_and_password(email, password).await?;
// Token is valid for 1 hour
// SDK automatically refreshes when needed

let token = user.id_token; // Always fresh
```

#### Auth State Listeners

Use async streams to monitor authentication state:

```rust
use futures::StreamExt;

let mut stream = auth.auth_state_changes().await;
while let Some(user_opt) = stream.next().await {
    match user_opt {
        Some(user) => println!("User signed in: {}", user.uid),
        None => println!("User signed out"),
    }
}
```

### Firestore

The Firestore module provides NoSQL document database operations.

#### Collections and Documents

```rust
// Reference a document
let doc_ref = firestore.document("users/alice");

// Reference a collection
let collection = firestore.collection("users");

// Nested collections
let posts = firestore.collection("users/alice/posts");
```

#### Queries

Build complex queries with filters and ordering:

```rust
use firebase_rust_sdk::firestore::{FilterCondition, OrderDirection};

let users = firestore.collection("users")
    .query()
    .where_filter(FilterCondition::GreaterThan("age".into(), json!(18)))
    .where_filter(FilterCondition::Equal("active".into(), json!(true)))
    .order_by("name", OrderDirection::Ascending)
    .limit(10)
    .get()
    .await?;
```

#### Compound Filters

Combine filters with And/Or logic:

```rust
// (age > 18 AND age < 65) OR status = "vip"
let filter = FilterCondition::Or(vec![
    FilterCondition::And(vec![
        FilterCondition::GreaterThan("age".into(), json!(18)),
        FilterCondition::LessThan("age".into(), json!(65)),
    ]),
    FilterCondition::Equal("status".into(), json!("vip")),
]);

let results = firestore.collection("users")
    .query()
    .where_filter(filter)
    .get()
    .await?;
```

#### Pagination

Use cursor-based pagination:

```rust
// First page
let first_page = firestore.collection("products")
    .query()
    .order_by("price", OrderDirection::Ascending)
    .limit(10)
    .get()
    .await?;

// Next page (start after last item)
let last_price = first_page.last()
    .and_then(|d| d.data.as_ref()?.get("price"));

if let Some(price) = last_price {
    let next_page = firestore.collection("products")
        .query()
        .order_by("price", OrderDirection::Ascending)
        .start_after(vec![price.clone()])
        .limit(10)
        .get()
        .await?;
}
```

#### Transactions

Atomic read-modify-write operations:

```rust
firestore.run_transaction(|mut txn| async move {
    // All reads must happen first
    let doc = txn.get("counters/visits").await?;
    let count = doc.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
    
    // Then perform writes
    txn.set("counters/visits", json!({"count": count + 1}));
    Ok(())
}).await?;
```

#### Real-time Listeners

Subscribe to document/query changes:

```rust
use futures::StreamExt;

// Listen to a document
let (registration, mut stream) = firestore
    .add_document_snapshot_listener("users/alice")
    .await?;

tokio::spawn(async move {
    while let Some(result) = stream.next().await {
        match result {
            Ok(snapshot) => println!("Document updated: {:?}", snapshot.data),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
});

// Listener auto-removed when registration is dropped
```

## Complete Examples

### OAuth Authentication

```rust
use firebase_rust_sdk::auth::Credential;

// Google sign-in
let google_cred = Credential::Google {
    id_token: Some("google_id_token".to_string()),
    access_token: Some("google_access_token".to_string()),
};
let result = auth.sign_in_with_credential(google_cred).await?;
println!("Signed in: {}", result.user.email);
println!("Provider: {}", result.additional_user_info.provider_id);

// Facebook sign-in
let facebook_cred = Credential::Facebook {
    access_token: "facebook_access_token".to_string(),
};
let result = auth.sign_in_with_credential(facebook_cred).await?;

// GitHub sign-in
let github_cred = Credential::GitHub {
    token: "github_token".to_string(),
};
let result = auth.sign_in_with_credential(github_cred).await?;

// Generic OAuth (e.g., Apple)
let oauth_cred = Credential::OAuth {
    provider_id: "apple.com".to_string(),
    id_token: Some("apple_id_token".to_string()),
    access_token: None,
    raw_nonce: Some("nonce".to_string()),
};
let result = auth.sign_in_with_credential(oauth_cred).await?;
```

### User Management

```rust
use firebase_rust_sdk::auth::UserProfile;

// Update profile
let profile = UserProfile {
    display_name: Some("Alice Smith".to_string()),
    photo_url: Some("https://example.com/photo.jpg".to_string()),
};
user.update_profile(profile).await?;

// Update email
user.update_email("newemail@example.com").await?;

// Update password
user.update_password("newpassword123").await?;

// Delete account
user.delete().await?;

// Send password reset
auth.send_password_reset_email("user@example.com").await?;
```

### Batch Operations

```rust
use firebase_rust_sdk::firestore::types::WriteBatch;

let mut batch = WriteBatch::new();

batch
    .set("users/alice", json!({"name": "Alice", "age": 30}))
    .update("users/bob", json!({"age": 31}))
    .delete("users/charlie");

firestore.commit_batch(batch).await?;
```

### Custom Token Authentication

```rust
// Sign in with JWT token generated by your server
let result = auth.sign_in_with_custom_token("your_server_jwt").await?;
println!("Custom auth user: {}", result.user.uid);
```

### Offline Persistence (API only, implementation pending)

```rust
use firebase_rust_sdk::firestore::types::{Settings, Source};

// Configure persistence
let mut settings = Settings::new();
settings.persistence_enabled = true;
settings.cache_size_bytes = Settings::CACHE_SIZE_UNLIMITED;

// Note: This will panic with todo!() until persistence is implemented
// firestore.set_settings(settings).await?;

// Network control (API defined)
// firestore.disable_network().await?;  // Offline mode
// firestore.enable_network().await?;   // Back online

// Source-based reads (API defined)
// let cached = firestore.get_document_with_source("users/alice", Source::Cache).await?;
// let fresh = firestore.get_document_with_source("users/alice", Source::Server).await?;
```

## Error Handling

All operations return `Result<T, FirebaseError>`:

```rust
use firebase_rust_sdk::error::{FirebaseError, AuthError, FirestoreError};

match auth.sign_in_with_email_and_password(email, password).await {
    Ok(user) => println!("Success: {}", user.uid),
    Err(FirebaseError::Auth(AuthError::InvalidEmail)) => {
        eprintln!("Invalid email format");
    }
    Err(FirebaseError::Auth(AuthError::WrongPassword)) => {
        eprintln!("Incorrect password");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Configuration

### Firebase Project Setup

1. Create a Firebase project at https://console.firebase.google.com/
2. Enable Authentication:
   - Go to Authentication → Sign-in method
   - Enable Email/Password and/or OAuth providers
3. Create Firestore database:
   - Go to Firestore Database
   - Create database in test/production mode
4. Get credentials:
   - Project Settings → Web API Key
   - Note your Project ID

### Environment Variables (for integration tests)

Create `.env` file:

```bash
FIREBASE_API_KEY=AIzaSyC...
FIREBASE_PROJECT_ID=my-project-id
TEST_USER_EMAIL=test@example.com
TEST_USER_PASSWORD=TestPassword123!
```

## Next Steps

- **[API Reference](API_REFERENCE.md)** - Complete API documentation
- **[Architecture](ARCHITECTURE.md)** - Design decisions and patterns
- **[Testing](TESTING.md)** - Integration tests setup
- **[Development](DEVELOPMENT.md)** - Contributing and porting guide

## Dependencies

```toml
[dependencies]
tokio = { version = "1.40", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2.0"
once_cell = "1.20"
futures = "0.3"
async-stream = "0.3"
rand = "0.8"
uuid = { version = "1.11", features = ["v4"] }
```

## Platform-Specific Notes

### WASM

Compiles for WASM but persistence requires IndexedDB implementation:

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["Window", "IndexedDb"] }
```

### Mobile (iOS/Android)

Use Rust mobile toolchains:

```bash
# iOS
rustup target add aarch64-apple-ios

# Android
rustup target add aarch64-linux-android
```

## Troubleshooting

### "Invalid API key" error
- Verify API key in Firebase Console → Project Settings
- Check for extra spaces in configuration

### "Permission denied" (Firestore)
- Check Firestore security rules
- Ensure user is authenticated for protected operations

### "Network error"
- Verify internet connection
- Check Firebase project status
- Confirm API key is for correct project

### Tests failing
- See [TESTING.md](TESTING.md) for integration test setup
- Run with `--nocapture` for detailed output
- Use `--test-threads=1` for sequential execution
