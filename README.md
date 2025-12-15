# Firebase Rust SDK

Port of Firebase C++ SDK (Auth + Firestore modules) to idiomatic Rust.

## Quick Start

1. **Read the manual:** [IMPLEMENTATION_MANUAL.md](IMPLEMENTATION_MANUAL.md) - Complete implementation guide
2. **Review analysis:** [QUICK_SUMMARY.md](QUICK_SUMMARY.md) - One-page overview
3. **Check details:** [RUST_PORTING_ANALYSIS.md](RUST_PORTING_ANALYSIS.md) - Detailed porting strategy

## Implementation Status

✅ **Phase 3 In Progress** - Advanced features

**Completed:**
- Error types (FirebaseError, AuthError, FirestoreError)
- Auth singleton with email/password authentication
- Anonymous authentication
- Password reset email
- Automatic token refresh with expiration tracking
- User account management (update_password, update_email, delete, update_profile)
- Auth state change listeners (async streams)
- Firestore initialization with singleton pattern
- Firestore document operations (Get, Set, Update, Delete)
- Firestore query operations (filters, ordering, limits)
- **Query pagination (start_at, start_after, end_at, end_before)**
- CollectionReference::add() with auto-generated IDs
- WriteBatch for atomic multi-document operations
- **Transactions for atomic read-modify-write operations**
- **Real-time snapshot listeners for documents and queries**
- **OAuth authentication providers (Google, Facebook, GitHub)**
- **Custom token authentication**

**Tests:** 85 tests passing (+2 custom token tests)

See [IMPLEMENTATION_MANUAL.md](IMPLEMENTATION_MANUAL.md) for detailed roadmap.

## Examples

### Authentication

```rust
use firebase_rust_sdk::Auth;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth = Auth::get_auth("YOUR_API_KEY").await?;
    
    // Sign in with email/password
    let user = auth.sign_in_with_email_and_password("user@example.com", "password").await?;
    println!("Signed in: {}", user.uid);
    
    // Or sign in anonymously
    let anon_user = auth.sign_in_anonymously().await?;
    println!("Anonymous user: {}", anon_user.user.uid);
    
    // OAuth sign-in with Google
    use firebase_rust_sdk::auth::Credential;
    let google_credential = Credential::Google {
        id_token: Some("google_id_token".to_string()),
        access_token: Some("google_access_token".to_string()),
    };
    let oauth_result = auth.sign_in_with_credential(google_credential).await?;
    println!("OAuth user: {}", oauth_result.user.uid);
    println!("Provider: {}", oauth_result.additional_user_info.provider_id);
    
    // OAuth sign-in with Facebook
    let facebook_credential = Credential::Facebook {
        access_token: "facebook_access_token".to_string(),
    };
    let fb_result = auth.sign_in_with_credential(facebook_credential).await?;
    
    // OAuth sign-in with GitHub
    let github_credential = Credential::GitHub {
        token: "github_token".to_string(),
    };
    let gh_result = auth.sign_in_with_credential(github_credential).await?;
    
    // Generic OAuth provider (e.g., Apple)
    let oauth_credential = Credential::OAuth {
        provider_id: "apple.com".to_string(),
        id_token: Some("apple_id_token".to_string()),
        access_token: None,
        raw_nonce: Some("nonce".to_string()),
    };
    let result = auth.sign_in_with_credential(oauth_credential).await?;
    
    // Custom token (server-generated JWT)
    let custom_result = auth.sign_in_with_custom_token("your_server_jwt_token").await?;
    println!("Custom token user: {}", custom_result.user.uid);
    
    // Send password reset email
    auth.send_password_reset_email("user@example.com").await?;
    println!("Password reset email sent");
    
    // Update user profile
    let profile = UserProfile {
        display_name: Some("Alice Smith".to_string()),
        photo_url: Some("https://example.com/photo.jpg".to_string()),
    };
    user.update_profile(profile).await?;
    
    // Listen to auth state changes
    let mut stream = auth.auth_state_changes().await;
    while let Some(user_opt) = stream.next().await {
        match user_opt {
            Some(user) => println!("User signed in: {}", user.uid),
            None => println!("User signed out"),
        }
    }
    
    Ok(())
}
```

### Firestore Queries & Batch Operations

```rust
use firebase_rust_sdk::firestore::{Firestore, types::WriteBatch, FilterCondition, OrderDirection};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let firestore = Firestore::get_firestore("my-project").await?;
    
    // Add document with auto-generated ID
    let doc_ref = firestore.collection("users")
        .add(json!({
            "name": "Alice",
            "age": 30,
            "email": "alice@example.com"
        }))
        .await?;
    println!("Created document: {}", doc_ref.path);
    
    // Batch write operations (atomic)
    let mut batch = WriteBatch::new();
    batch.set("users/bob", json!({"name": "Bob", "age": 25}))
         .update("users/alice", json!({"age": 31}))
         .delete("users/charlie");
    firestore.commit_batch(batch).await?;
    println!("Batch committed successfully");
    
    // Query with pagination
    let docs = firestore.collection("users")
        .query()
        .where_filter(FilterCondition::GreaterThan("age".to_string(), json!(18)))
        .where_filter(FilterCondition::Equal("active".to_string(), json!(true)))
        .order_by("age", OrderDirection::Ascending)
        .start_after(vec![json!(25)])  // Start after age 25 (exclusive)
        .limit(10)
        .get()
        .await?;
    
    for doc in docs {
        println!("Document: {} (age: {})", doc.reference.id(), doc.data.as_ref().unwrap()["age"]);
    }
    
    // Next page using end value from previous results
    let last_age = docs.last().and_then(|d| d.data.as_ref()?.get("age"));
    if let Some(age) = last_age {
        let next_page = firestore.collection("users")
            .query()
            .order_by("age", OrderDirection::Ascending)
            .start_after(vec![age.clone()])
            .limit(10)
            .get()
            .await?;
        println!("Next page: {} documents", next_page.len());
    }
    
    Ok(())
}
```

### Transactions

```rust
use firebase_rust_sdk::firestore::Firestore;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let firestore = Firestore::get_firestore_with_database_and_key(
        "my-project",
        "(default)",
        "YOUR_API_KEY"
    ).await?;
    
    // Atomic counter increment with retry
    firestore.run_transaction(|mut txn| async move {
        // All reads must happen first
        let doc = txn.get("counters/visits").await?;
        let count = doc.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
        
        // Then perform writes
        txn.set("counters/visits", json!({"count": count + 1}));
        Ok(())
    }).await?;
    
    // Custom retry attempts
    firestore.run_transaction_with_options(|mut txn| async move {
        let balance_doc = txn.get("accounts/alice").await?;
        let balance = balance_doc.get("balance").and_then(|v| v.as_f64()).unwrap_or(0.0);
        
        if balance < 100.0 {
            return Err(FirebaseError::Firestore(
                FirestoreError::InvalidArgument("Insufficient funds".to_string())
            ));
        }
        
        txn.update("accounts/alice", json!({"balance": balance - 100.0}))
           .update("accounts/bob", json!({"balance": balance_doc.get("balance").unwrap_or(&json!(0.0)).as_f64().unwrap_or(0.0) + 100.0}));
        
        Ok(())
    }, 10).await?; // Retry up to 10 times
    
    Ok(())
}
```

### Real-time Snapshot Listeners

```rust
use firebase_rust_sdk::firestore::Firestore;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let firestore = Firestore::get_firestore("my-project").await?;
    
    // Listen to a single document
    let (registration, mut stream) = firestore
        .add_document_snapshot_listener("users/alice")
        .await?;
    
    tokio::spawn(async move {
        while let Some(result) = stream.next().await {
            match result {
                Ok(snapshot) => {
                    if snapshot.exists() {
                        println!("Document updated: {:?}", snapshot.data);
                    } else {
                        println!("Document deleted");
                    }
                }
                Err(e) => eprintln!("Listener error: {}", e),
            }
        }
    });
    
    // Listen to a query
    let query = firestore.collection("users")
        .query()
        .where_filter(FilterCondition::Equal("active".to_string(), json!(true)))
        .order_by("name", OrderDirection::Ascending);
    
    let (query_registration, mut query_stream) = firestore
        .add_query_snapshot_listener(query)
        .await?;
    
    while let Some(result) = query_stream.next().await {
        match result {
            Ok(snapshot) => {
                println!("Query updated: {} documents", snapshot.len());
                for doc in snapshot.documents {
                    println!("  - {}: {:?}", doc.reference.id(), doc.data);
                }
            }
            Err(e) => eprintln!("Query listener error: {}", e),
        }
    }
    
    // Remove listener explicitly (or drop registration to auto-remove)
    registration.remove();
    
    Ok(())
}
```

## Project Structure

```
src/
├── lib.rs              # Public API
├── error.rs            # Error types
├── auth/               # Authentication module
│   ├── types.rs
│   ├── auth.rs
│   └── providers/
└── firestore/          # Firestore module
    ├── types.rs
    ├── document.rs
    └── query.rs

analysis_output/        # Dependency analysis data
├── implementation_plan.json
└── api_reports/        # 399 per-API reports with C++ locations
```

## Analysis Data

All dependency analysis complete:
- 3,393 components analyzed
- 7,731 dependencies mapped
- 399 public APIs cataloged with file locations
- 32 implementation layers identified

Use `analysis_output/api_reports/*.json` to find C++ implementations.

## Development

```bash
# Build
cargo build

# Test
cargo test

# Format
cargo fmt

# Lint
cargo clippy
```

## Timeline

**Total:** 12-16 weeks (3-4 months)
- Foundation: 2-3 weeks
- Auth Module: 3-4 weeks  
- Firestore Module: 4-6 weeks
- Testing & Polish: 2-3 weeks

## License

Apache 2.0
