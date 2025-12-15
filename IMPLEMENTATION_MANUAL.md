# Firebase Rust SDK - Implementation Manual

## Quick Reference

**Project Goal:** Port Firebase C++ SDK (Auth + Firestore) to idiomatic Rust  
**Approach:** Bottom-up implementation with top-down API design

---

## Design Decisions

### Questions Requiring Input

Before implementing core functionality, clarify these design decisions:

1. **Auth Singleton Management:**
   - Global singleton with lazy_static/once_cell?
   - Or explicit Auth instance creation per API key?
   - How to handle multiple Firebase projects in one app?

2. **Token Refresh Strategy:**
   - Automatic background refresh vs on-demand?
   - Handle token expiration during long-running operations?
   - Expose refresh token to users or keep internal?

3. **Listener/Stream Semantics:**
   - Auth state changes: tokio::sync::watch or async-stream?
   - Hot (broadcast) or cold (unicast) streams?
   - Backpressure handling for Firestore real-time updates?

4. **Error Retry Policy:**
   - Automatic retry with exponential backoff?
   - User-controlled retry vs built-in?
   - Max retry attempts and timeout config?

5. **Offline Support:**
   - Local caching for Firestore in v1?
   - Offline writes and conflict resolution?
   - Or keep online-only initially?

6. **API Surface:**
   - Follow C++ API closely for familiarity?
   - Or redesign for Rust idioms (builders, type states)?
   - Include all C++ methods or subset?

7. **HTTP Client:**
   - Use reqwest (current dependency)?
   - Support custom client injection for testing?
   - Connection pooling and timeout configuration?

8. **Async Runtime:**
   - Require tokio (current approach)?
   - Or runtime-agnostic with async-std compat?

### Decisions Made

**1. Auth Singleton Management:** ✅ DECIDED
- Follow C++ pattern: One Auth instance per API key (like one per App)
- C++ Implementation: `Auth::GetAuth(app)` maintains global `std::map<App*, Auth*>`
- Rust Implementation: Use `once_cell::sync::Lazy<Mutex<HashMap<String, Arc<Auth>>>>>`
- Key insight from C++: `auth/src/auth.cc:65` - global map `g_auths` with mutex
- User calls `Auth::get_auth(api_key)` which returns existing or creates new

**2. Token Refresh Strategy:** ✅ DECIDED
- Follow C++ implementation patterns
- Need to investigate: `auth/src/desktop/auth_desktop.cc` for token refresh logic
- Likely on-demand refresh when token expired

**3. Listener/Stream Semantics:** ✅ DECIDED
- Use `async-stream` for auth state changes and Firestore snapshots
- MPSC (Multi-Producer Single-Consumer) pattern with tokio channels
- C++ Reference: `auth.h:650` - IdTokenListener, `auth.h:610` - AuthStateListener

**4. Error Retry Policy:** ✅ DECIDED
- Follow C++ implementation patterns
- Generally user-controlled, but check C++ for any automatic retries

**5. Offline Support:** ✅ DECIDED
- Online-only for v1
- Leave `todo!()` macros for future offline cache implementation
- Document offline methods but don't implement yet

**6. API Surface:** ✅ DECIDED
- Hybrid approach: C++ direct translation when it matches Rust idioms
- Use Rust patterns when they're more idiomatic (builders, type states, Result)
- Example: `Auth::GetAuth(app)` → `Auth::get_auth(api_key)`
- Keep method names similar but Rust-cased: `SignInWithEmail` → `sign_in_with_email`

**7. HTTP Client:** ✅ DECIDED
- Use `reqwest` (already in dependencies)
- Single shared client per Auth instance with connection pooling
- Default timeouts: 30s for most operations, 60s for sign-in

**8. Async Runtime:** ✅ DECIDED
- Require `tokio` (simplifies implementation)
- All async functions assume tokio runtime
- Add runtime feature flag if needed later

---

## Core Principles

### 1. Rust Idioms Over C++ Direct Port
- ❌ `Future<T>` with callbacks → ✅ `async fn() -> Result<T>`
- ❌ `AddListener(callback)` → ✅ `fn stream() -> impl Stream<Item=T>`
- ❌ Error codes → ✅ `Result<T, FirebaseError>`
- ❌ `shared_ptr<T>` → ✅ `Arc<RwLock<T>>`
- ❌ Nullable pointers → ✅ `Option<T>`

### 2. Code Style (STRICT)
```rust
// ✅ Prefer early returns with let-else
let Some(user) = get_user() else {
    return Err(FirebaseError::NotAuthenticated);
};

// ✅ Prefer if-let for Result<(), E>
if let Err(e) = validate_input() {
    return Err(e.into());
}

// ✅ Prefer match for extracting values
let user = match auth.current_user() {
    Err(e) => return Err(e),
    Ok(u) => u,
};

// ✅ Error/exceptional cases first
match result {
    Err(e) => return Err(e),
    Ok(value) => {
        // Happy path
    }
}

// ✅ Prefer match over if-else (except for booleans)
let status = match response.status() {
    200 => Status::Ok,
    404 => Status::NotFound,
    _ => Status::Error,
};

// ✅ Early returns, flat control flow
fn process(data: &Data) -> Result<()> {
    if !data.is_valid() {
        return Err(Error::Invalid);
    }
    
    let Some(value) = data.value else {
        return Err(Error::Missing);
    };
    
    // Continue processing...
    Ok(())
}

// ✅ Prefer let/if let/match over method chaining when possible
// Good: Explicit control flow
let response = fetch_data().await;
let data = match response {
    Err(e) => return Err(e),
    Ok(d) => d,
};

// Avoid: Long method chains (harder to debug, less clear)
let data = fetch_data().await?.map(|x| x.value)?;

// Exception: Short, clear chains are OK
let text = s.trim().to_lowercase();
```

### 3. Implementation Strategy
- **Bottom-up:** Build from leaf nodes (0 dependencies) upward
- **Top-down API:** Design public API first with `todo!()` macros
- **Test-driven:** Write tests for each component before climbing up
- **Track progress:** Update implementation tracker after each component

### 4. Code Structure
- Follow C++ SDK structure where it makes sense
- Keep similar file organization: `auth/`, `firestore/`, `common/`
- Match C++ class names but use Rust naming conventions

---

## Project Structure

```
src/
├── lib.rs                  # Public API exports
├── error.rs                # FirebaseError, AuthError, FirestoreError
├── common/
│   ├── mod.rs
│   ├── future.rs          # Async utilities
│   └── cleanup.rs         # Drop guards, resource management
├── auth/
│   ├── mod.rs             # Re-exports
│   ├── types.rs           # User, Credential, AuthResult, etc.
│   ├── auth.rs            # Auth struct
│   ├── user.rs            # User methods
│   ├── providers/
│   │   ├── mod.rs
│   │   ├── email.rs       # EmailAuthProvider
│   │   ├── google.rs      # GoogleAuthProvider
│   │   └── oauth.rs       # FederatedOAuthProvider
│   └── internal/          # Private implementation details
│       └── auth_impl.rs
└── firestore/
    ├── mod.rs
    ├── types.rs           # FieldValue, Timestamp, GeoPoint
    ├── firestore.rs       # Firestore struct
    ├── document.rs        # DocumentReference, DocumentSnapshot
    ├── collection.rs      # CollectionReference
    ├── query.rs           # Query, Filter
    ├── transaction.rs     # Transaction, WriteBatch
    └── internal/
        └── firestore_impl.rs
```

---

## Implementation Tracker

Track what's been implemented to enable reuse:

```rust
// In implementation_tracker.rs
pub struct ImplementationStatus {
    pub component: &'static str,
    pub status: Status,
    pub location: &'static str,
    pub dependencies: &'static [&'static str],
}

pub enum Status {
    NotStarted,
    InProgress,
    Tested,
    Documented,
}

// Update after each implementation
pub const IMPLEMENTED: &[ImplementationStatus] = &[
    ImplementationStatus {
        component: "FirebaseError",
        status: Status::Documented,
        location: "src/error.rs",
        dependencies: &[],
    },
    // Add more as implemented...
];
```

---

## Dependencies

```toml
[dependencies]
tokio = { version = "1.40", features = ["full"] }
futures = "0.3"
async-stream = "0.3"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"

[dev-dependencies]
tokio-test = "0.4"
```

---

## Phase 1: Foundation ✅ COMPLETE

### Step 1: Error Types ✅ IMPLEMENTED
**File:** `src/error.rs`
**Status:** Complete with 8 tests passing

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FirebaseError {
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),
    
    #[error("Firestore error: {0}")]
    Firestore(#[from] FirestoreError),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid email")]
    InvalidEmail,
    
    #[error("Invalid password")]
    InvalidPassword,
    
    #[error("User not found")]
    UserNotFound,
    
    #[error("Invalid credential")]
    InvalidCredential,
    
    #[error("Not authenticated")]
    NotAuthenticated,
}

#[derive(Debug, Error)]
pub enum FirestoreError {
    #[error("Document not found")]
    NotFound,
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Invalid field value")]
    InvalidFieldValue,
    
    #[error("Transaction failed")]
    TransactionFailed,
}
```

**Test:** `tests/error_tests.rs`
```rust
#[test]
fn test_error_conversion() {
    let auth_err = AuthError::InvalidEmail;
    let firebase_err: FirebaseError = auth_err.into();
    assert!(matches!(firebase_err, FirebaseError::Auth(_)));
}
```

**Location in C++:** `auth/src/include/firebase/auth/types.h` (error codes)

### Step 2: Basic Auth Types ✅ IMPLEMENTED
**File:** `src/auth/types.rs`
**Status:** Complete with 5 tests passing

**C++ Reference:** `auth/src/include/firebase/auth/user.h:498`

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// User profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub uid: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub photo_url: Option<String>,
    pub phone_number: Option<String>,
    pub is_anonymous: bool,
    pub is_email_verified: bool,
    pub metadata: UserMetadata,
    pub provider_data: Vec<UserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMetadata {
    pub creation_timestamp: DateTime<Utc>,
    pub last_sign_in_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub uid: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub photo_url: Option<String>,
    pub provider_id: String,
}

#[derive(Debug, Clone)]
pub struct Credential {
    provider: String,
    token: String,
}

impl Credential {
    pub fn provider(&self) -> &str {
        &self.provider
    }
    
    pub fn is_valid(&self) -> bool {
        !self.provider.is_empty() && !self.token.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct AuthResult {
    pub user: User,
    pub additional_user_info: Option<AdditionalUserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdditionalUserInfo {
    pub provider_id: String,
    pub username: Option<String>,
    pub is_new_user: bool,
}
```

**Test:** `tests/auth_types_tests.rs`

### Step 3: Firestore Types ✅ IMPLEMENTED
**File:** `src/firestore/types.rs`
**Status:** Complete with 11 tests passing (using serde_json::Value instead of custom FieldValue)

**C++ Reference:** `firestore/src/include/firebase/firestore/field_value.h`

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Double(f64),
    String(String),
    Bytes(Vec<u8>),
    Timestamp(DateTime<Utc>),
    GeoPoint(GeoPoint),
    Array(Vec<FieldValue>),
    Map(HashMap<String, FieldValue>),
    Reference(String), // Document path
}

impl FieldValue {
    pub fn is_null(&self) -> bool {
        matches!(self, FieldValue::Null)
    }
    
    pub fn as_string(&self) -> Option<&str> {
        match self {
            FieldValue::String(s) => Some(s),
            _ => None,
        }
    }
    
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            FieldValue::Integer(i) => Some(*i),
            _ => None,
        }
    }
    
    // ... more convenience methods
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeoPoint {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamp {
    pub seconds: i64,
    pub nanos: i32,
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Default,
    Server,
    Cache,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataChanges {
    Exclude,
    Include,
}
```

---

## Phase 2: Core Infrastructure ✅ COMPLETE

### Step 4: Auth Singleton ✅ IMPLEMENTED
**File:** `src/auth/auth.rs`
**Status:** Complete with auth state listeners and 11 tests passing

**C++ Reference:** `auth/src/desktop/auth_desktop.cc:356`

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

static AUTH_INSTANCE: Lazy<Arc<RwLock<Option<Auth>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(None)));

pub struct Auth {
    inner: Arc<AuthInner>,
}

struct AuthInner {
    api_key: String,
    current_user: RwLock<Option<User>>,
}

impl Auth {
    /// Get or create the Auth instance
    pub async fn get_auth(api_key: impl Into<String>) -> Result<Auth, FirebaseError> {
        let api_key = api_key.into();
        
        let mut instance = AUTH_INSTANCE.write().await;
        
        match instance.as_ref() {
            Some(auth) => Ok(auth.clone()),
            None => {
                let auth = Auth {
                    inner: Arc::new(AuthInner {
                        api_key,
                        current_user: RwLock::new(None),
                    }),
                };
                *instance = Some(auth.clone());
                Ok(auth)
            }
        }
    }
    
    pub async fn current_user(&self) -> Option<User> {
        self.inner.current_user.read().await.clone()
    }
    
    pub async fn sign_out(&self) -> Result<(), FirebaseError> {
        let mut user = self.inner.current_user.write().await;
        *user = None;
        Ok(())
    }
}

impl Clone for Auth {
    fn clone(&self) -> Self {
        Auth {
            inner: Arc::clone(&self.inner),
        }
    }
}
```

### Step 5: Email/Password Auth
**File:** `src/auth/auth.rs` (add methods)

**C++ Reference:** 
- SignIn: `auth/src/desktop/auth_desktop.cc:405`
- CreateUser: `auth/src/desktop/auth_desktop.cc:298`

```rust
impl Auth {
    pub async fn sign_in_with_email_and_password(
        &self,
        email: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> Result<User, FirebaseError> {
        let email = email.as_ref();
        let password = password.as_ref();
        
        // Validate inputs (error cases first)
        if email.is_empty() {
            return Err(AuthError::InvalidEmail.into());
        }
        
        if password.is_empty() {
            return Err(AuthError::InvalidPassword.into());
        }
        
        // Make API call
        let response = reqwest::Client::new()
            .post("https://identitytoolkit.googleapis.com/v1/accounts:signInWithPassword")
            .query(&[("key", &self.inner.api_key)])
            .json(&serde_json::json!({
                "email": email,
                "password": password,
                "returnSecureToken": true
            }))
            .send()
            .await?;
        
        // Handle errors first
        if !response.status().is_success() {
            return Err(AuthError::InvalidCredential.into());
        }
        
        let user_data: UserResponse = response.json().await?;
        let user = user_data.into_user();
        
        // Update current user
        let mut current = self.inner.current_user.write().await;
        *current = Some(user.clone());
        
        Ok(user)
    }
    
    pub async fn create_user_with_email_and_password(
        &self,
        email: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> Result<AuthResult, FirebaseError> {
        let email = email.as_ref();
        let password = password.as_ref();
        
        // Validation (error cases first)
        if email.is_empty() {
            return Err(AuthError::InvalidEmail.into());
        }
        
        if password.is_empty() {
            return Err(AuthError::InvalidPassword.into());
        }
        
        // API call
        let response = reqwest::Client::new()
            .post("https://identitytoolkit.googleapis.com/v1/accounts:signUp")
            .query(&[("key", &self.inner.api_key)])
            .json(&serde_json::json!({
                "email": email,
                "password": password,
                "returnSecureToken": true
            }))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(AuthError::InvalidCredential.into());
        }
        
        let user_data: UserResponse = response.json().await?;
        let user = user_data.into_user();
        
        // Update current user
        let mut current = self.inner.current_user.write().await;
        *current = Some(user.clone());
        
        Ok(AuthResult {
            user,
            additional_user_info: Some(AdditionalUserInfo {
                provider_id: "password".to_string(),
                username: None,
                is_new_user: true,
            }),
        })
    }
}
```

---

## Testing Strategy

### Unit Tests (per module)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_email_validation() {
        let auth = Auth::get_auth("test_key").await.unwrap();
        
        // Test error case first
        let result = auth.sign_in_with_email_and_password("", "password").await;
        assert!(matches!(result, Err(FirebaseError::Auth(AuthError::InvalidEmail))));
        
        // Test valid case
        // (requires mock server)
    }
}
```

### Integration Tests
```rust
// tests/integration_test.rs
#[tokio::test]
#[ignore] // Only run with actual Firebase project
async fn test_real_sign_in() {
    let auth = Auth::get_auth(env::var("FIREBASE_API_KEY").unwrap())
        .await
        .unwrap();
        
    let user = auth
        .sign_in_with_email_and_password("test@example.com", "password123")
        .await
        .unwrap();
        
    assert!(!user.uid.is_empty());
}
```

---

## Commit Strategy

After each component is tested and working:

```bash
# Example commits
git add src/error.rs tests/error_tests.rs
git commit -m "Add FirebaseError, AuthError, FirestoreError types

- Comprehensive error enum with thiserror
- Error conversions working
- Tests passing"

git add src/auth/types.rs tests/auth_types_tests.rs
git commit -m "Add Auth types: User, Credential, AuthResult

Location: auth/src/include/firebase/auth/user.h:498
- User struct with metadata
- Credential validation
- Tests for all types"
```

---

## Top-Down API Design (To be refined as we implement)

**File:** `src/lib.rs`

```rust
// Public API - starts with todo!() until implemented from bottom

pub mod error;
pub mod auth;
pub mod firestore;

// Re-exports
pub use error::{FirebaseError, AuthError, FirestoreError};
pub use auth::{Auth, User, Credential, AuthResult};
pub use firestore::{
    Firestore, DocumentReference, CollectionReference,
    Query, FieldValue, Transaction, WriteBatch,
};

/// Initialize Firebase (if needed globally)
pub async fn initialize(config: FirebaseConfig) -> Result<(), FirebaseError> {
    todo!("Initialize Firebase with config")
}
```

---

## Analysis Data Reference

- **Full dependency graph:** `analysis_output/implementation_plan.json`
- **Per-API reports:** `analysis_output/api_reports/*.json`
- **C++ locations:** All APIs have `file_path` and `line_number` in reports

---

## Quick Reference Commands

```bash
# Find C++ implementation
jq '.location' analysis_output/api_reports/firebase_auth_Auth_SignInWithCredential.json

# Run tests
cargo test

# Run specific test
cargo test test_email_validation

# Check implementation status
rg "TODO|FIXME|todo!" src/

# Commit progress
git add -A && git commit -m "Implement [component]"
```

---

## Next Steps

1. ✅ Read this manual
2. ✅ Implement Phase 1 (Foundation) - Complete with 24 tests passing
3. ✅ Test thoroughly before moving to Phase 2 - All tests passing
4. ✅ Implement Phase 2 (Core Infrastructure) - Complete with 47 tests total
5. ⬜ Implement Phase 3: Query operations, additional auth methods, transactions
6. ⬜ Update tracker as components are completed
7. ⬜ Refine top-level API based on learnings

**Remember:** Bottom-up implementation, test each piece, commit regularly!
