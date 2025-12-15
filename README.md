# Firebase Rust SDK

Port of Firebase C++ SDK (Auth + Firestore modules) to idiomatic Rust.

## Quick Start

1. **Read the manual:** [IMPLEMENTATION_MANUAL.md](IMPLEMENTATION_MANUAL.md) - Complete implementation guide
2. **Review analysis:** [QUICK_SUMMARY.md](QUICK_SUMMARY.md) - One-page overview
3. **Check details:** [RUST_PORTING_ANALYSIS.md](RUST_PORTING_ANALYSIS.md) - Detailed porting strategy

## Implementation Status

✅ **Phase 3 In Progress** - Auth features and query operations

**Completed:**
- Error types (FirebaseError, AuthError, FirestoreError)
- Auth singleton with email/password authentication
- Anonymous authentication
- Password reset email
- **NEW:** Automatic token refresh with expiration tracking
- Auth state change listeners (async streams)
- Firestore initialization with singleton pattern
- Firestore document operations (Get, Set, Update, Delete)
- Firestore query operations (filters, ordering, limits, cursors)

**Tests:** 57 tests passing

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
    
    // Send password reset email
    auth.send_password_reset_email("user@example.com").await?;
    println!("Password reset email sent");
    
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

### Firestore Queries

```rust
use firebase_rust_sdk::firestore::{Firestore, FilterCondition, OrderDirection};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let firestore = Firestore::get_firestore("my-project").await?;
    
    // Query documents
    let docs = firestore.collection("users")
        .query()
        .where_filter(FilterCondition::GreaterThan("age".to_string(), json!(18)))
        .where_filter(FilterCondition::Equal("active".to_string(), json!(true)))
        .order_by("age", OrderDirection::Ascending)
        .limit(10)
        .get()
        .await?;
    
    for doc in docs {
        println!("Document: {}", doc.reference.id());
    }
    
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
