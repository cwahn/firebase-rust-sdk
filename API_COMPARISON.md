# Firebase Rust SDK vs C++ SDK - Public API Comparison

**Date:** December 16, 2025  
**Version:** 0.1.0-alpha.2 (in progress)  
**Last Updated:** December 16, 2025 - Added SendEmailVerificationBeforeUpdatingEmail, CollectionGroup, RunTransaction

## Overview

This document compares the public APIs of the Firebase Rust SDK with the original Firebase C++ SDK to identify implemented features and gaps.

---

## 📊 Summary Statistics

### Firebase Authentication
- **✅ Implemented:** 18/35 major methods (~51%)
- **❌ Missing:** 17/35 major methods (~49%)

### Cloud Firestore
- **✅ Implemented:** 23/45 major methods (~51%)
- **❌ Missing:** 22/45 major methods (~49%)

---

## 🔐 Firebase Authentication API

### Auth Class

#### ✅ Implemented (18 methods)

| Rust Method | C++ Reference | Notes |
|------------|---------------|-------|
| `Auth::get_auth(app)` | `Auth::GetAuth(app)` | Singleton pattern with global map |
| `current_user()` | `current_user()` | Returns `Option<Arc<User>>` |
| `sign_out()` | `SignOut()` | Clears current user |
| `api_key()` | Internal accessor | Helper method |
| `app()` | `app()` | Returns reference to App |
| `language_code()` | `language_code()` | **⚠️ Stub** - Returns empty string |
| `set_language_code()` | `set_language_code()` | **⚠️ Stub** - No-op |
| `use_app_language()` | `UseAppLanguage()` | **⚠️ Stub** - No-op |
| `use_emulator()` | `UseEmulator()` | **⚠️ Stub** - No-op |
| `auth_state_changes()` | `AddAuthStateListener()` | Returns Stream (idiomatic Rust) |
| `sign_in_with_email_and_password()` | `SignInWithEmailAndPassword()` | Full implementation |
| `create_user_with_email_and_password()` | `CreateUserWithEmailAndPassword()` | Full implementation |
| `sign_in_anonymously()` | `SignInAnonymously()` | Full implementation |
| `sign_in_with_credential()` | `SignInWithCredential()` | Supports OAuth providers |
| `sign_in_with_custom_token()` | `SignInWithCustomToken()` | Server-side JWT auth |
| `send_password_reset_email()` | `SendPasswordResetEmail()` | Full implementation |
| `fetch_providers_for_email()` | `FetchProvidersForEmail()` | Full implementation |

#### ❌ Missing (17 methods)

| C++ Method | Status | Priority |
|-----------|--------|----------|
| `SignInWithProvider()` | 🔴 Not implemented | High - OAuth flows |
| `LinkWithCredential()` | 🔴 Not implemented | Medium - Account linking |
| `UnlinkProvider()` | 🔴 Not implemented | Medium - Account unlinking |
| `ReauthenticateWithCredential()` | 🔴 Not implemented | Medium - Sensitive operations |
| `ConfirmPasswordReset()` | 🔴 Not implemented | High - Password reset flow |
| `VerifyPasswordResetCode()` | 🔴 Not implemented | High - Password reset flow |
| `ApplyActionCode()` | 🔴 Not implemented | Medium - Action codes |
| `CheckActionCode()` | 🔴 Not implemented | Medium - Action codes |
| `AddIdTokenListener()` | 🔴 Not implemented | Low - Token refresh events |
| `RemoveIdTokenListener()` | 🔴 Not implemented | Low - Token refresh events |
| `SetPersistence()` | 🔴 Not implemented | Medium - Offline support |
| `GetCredentialFromEmailLink()` | 🔴 Not implemented | Low - Email link auth |
| `IsEmailLink()` | 🔴 Not implemented | Low - Email link validation |
| `UpdateCurrentUser()` | 🔴 Not implemented | Low - Admin SDK feature |
| `SignInWithPopup()` | 🟡 Web only | N/A - Platform specific |
| `SignInWithRedirect()` | 🟡 Web only | N/A - Platform specific |

### User Class

#### ✅ Implemented (9 methods)

| Rust Method | C++ Reference | Notes |
|------------|---------------|-------|
| `get_id_token(force_refresh)` | `GetToken()` | Auto token refresh |
| `delete()` | `Delete()` | Account deletion |
| `update_email()` | `UpdateEmail()` | Full implementation |
| `update_password()` | `UpdatePassword()` | Full implementation |
| `update_profile()` | `UpdateUserProfile()` | Display name & photo URL |
| `reload()` | `Reload()` | Refresh user data |
| `reauthenticate()` | `Reauthenticate()` | Full implementation |
| `send_email_verification()` | `SendEmailVerification()` | Full implementation |
| `send_email_verification_before_updating_email()` | `SendEmailVerificationBeforeUpdatingEmail()` | **NEW** - Verify new email before update |

#### ❌ Missing (4 methods)

| C++ Method | Status | Priority |
|-----------|--------|----------|
| `LinkAndRetrieveDataWithCredential()` | 🔴 Not implemented | Medium |
| `ReauthenticateAndRetrieveData()` | 🔴 Not implemented | Medium |
| `Unlink()` | 🔴 Not implemented | Medium |
| `UpdatePhoneNumberCredential()` | 🔴 Not implemented | Low - Phone auth |
| `GetProviderData()` | ✅ Available as `provider_data` field | - |

### Credential Types

#### ✅ Implemented (6 providers)

| Rust Type | C++ Reference | Notes |
|----------|---------------|-------|
| `Credential::Email` | `EmailAuthProvider::GetCredential()` | Email/password |
| `Credential::Google` | `GoogleAuthProvider::GetCredential()` | OAuth 2.0 |
| `Credential::Facebook` | `FacebookAuthProvider::GetCredential()` | OAuth 2.0 |
| `Credential::GitHub` | `GitHubAuthProvider::GetCredential()` | OAuth 2.0 |
| `Credential::OAuth` | `OAuthProvider::GetCredential()` | Generic OAuth |
| `Credential::Anonymous` | Internal | Auto-generated |

#### ❌ Missing (5 providers)

| C++ Provider | Status | Priority |
|-------------|--------|----------|
| `PhoneAuthProvider` | 🔴 Not implemented | Medium - SMS auth |
| `TwitterAuthProvider` | 🔴 Not implemented | Low - Legacy OAuth 1.0 |
| `MicrosoftAuthProvider` | 🔴 Not implemented | Medium - Enterprise |
| `YahooAuthProvider` | 🔴 Not implemented | Low - Uncommon |
| `GameCenterAuthProvider` | 🔴 Not implemented | Low - iOS only |

---

## 🗄️ Cloud Firestore API

### Firestore Class

#### ✅ Implemented (8 methods)

| Rust Method | C++ Reference | Notes |
|------------|---------------|-------|
| `Firestore::new()` | `GetInstance()` | gRPC client with TLS |
| `project_id()` | `project_id()` | Accessor |
| `database_id()` | `database_id()` | Accessor |
| `document(path)` | `Document()` | Returns DocumentReference |
| `collection(path)` | `Collection()` | Returns CollectionReference |
| `batch()` | `batch()` | Returns WriteBatch |
| `collection_group(id)` | `CollectionGroup()` | **NEW** - Cross-collection queries |
| `run_transaction(fn)` | `RunTransaction()` | **NEW** - Atomic operations with retries |

#### ❌ Missing (7 methods)

| C++ Method | Status | Priority |
|-----------|--------|----------|
| `EnableNetwork()` | 🔴 Not implemented | Medium - Online/offline mode |
| `DisableNetwork()` | 🔴 Not implemented | Medium - Online/offline mode |
| `WaitForPendingWrites()` | 🔴 Not implemented | Medium - Sync control |
| `Terminate()` | 🔴 Not implemented | Low - Cleanup |
| `ClearPersistence()` | 🔴 Not implemented | Low - Cache management |
| `LoadBundle()` | 🔴 Not implemented | Low - Data bundles |
| `NamedQuery()` | 🔴 Not implemented | Low - Data bundles |

### Settings

#### ✅ Implemented (3 properties)

| Rust Property | C++ Reference | Notes |
|--------------|---------------|-------|
| `host` | `set_host()` | Firestore endpoint |
| `ssl_enabled` | `set_ssl_enabled()` | TLS configuration |
| `cache_size_bytes` | `set_cache_size_bytes()` | **⚠️ Persistence not implemented** |

#### ❌ Missing (1 property)

| C++ Property | Status | Priority |
|-------------|--------|----------|
| `persistence_enabled` | 🟡 API exists | **Stub** - Offline persistence pending |

### DocumentReference Class

#### ✅ Implemented (7 methods)

| Rust Method | C++ Reference | Notes |
|------------|---------------|-------|
| `id()` | `id()` | Document ID |
| `path` | `path()` | Full path property |
| `set(data)` | `Set()` | Replace document |
| `update(data)` | `Update()` | Partial update |
| `delete()` | `Delete()` | Remove document |
| `get()` | `Get()` | Fetch snapshot |
| `listen()` | `AddSnapshotListener()` | Real-time updates via Stream |

#### ❌ Missing (2 methods)

| C++ Method | Status | Priority |
|-----------|--------|----------|
| `Collection()` | 🔴 Not implemented | Medium - Subcollections |
| `Parent()` | 🟡 Partial | Helper `parent_path()` exists |

### CollectionReference Class

#### ✅ Implemented (3 methods)

| Rust Method | C++ Reference | Notes |
|------------|---------------|-------|
| `path` | `path()` | Collection path property |
| `add(data)` | `Add()` | Auto-generated document ID |
| `document(id)` | `Document()` | Get document reference |

#### ❌ Missing (1 method)

| C++ Method | Status | Priority |
|-----------|--------|----------|
| `Parent()` | 🟡 Partial | Helper `parent_path()` exists |

### Query Class (DocumentReference & CollectionReference)

#### ✅ Implemented (10 methods)

| Rust Method | C++ Reference | Notes |
|------------|---------------|-------|
| `where_field(filter)` | `Where()` / `WhereEqualTo()` etc. | Unified filter interface |
| `order_by(field, direction)` | `OrderBy()` | ASC/DESC sorting |
| `limit(count)` | `Limit()` | Result limiting |
| `limit_to_last(count)` | `LimitToLast()` | Last N results |
| `start_at(values)` | `StartAt()` | Pagination cursor |
| `start_after(values)` | `StartAfter()` | Pagination cursor |
| `end_at(values)` | `EndAt()` | Pagination cursor |
| `end_before(values)` | `EndBefore()` | Pagination cursor |
| `get()` | `Get()` | Execute query |
| `listen()` | `AddSnapshotListener()` | Real-time query updates |

#### ❌ Missing (2 methods)

| C++ Method | Status | Priority |
|-----------|--------|----------|
| `Count()` | 🔴 Not implemented | Medium - Aggregate queries |
| `AggregateQuery()` | 🔴 Not implemented | Low - Advanced aggregation |

### Filter Operations

#### ✅ Implemented (12 operators)

| Rust Operator | C++ Reference | Notes |
|--------------|---------------|-------|
| `FilterCondition::Equal` | `Filter::EqualTo()` | field == value |
| `FilterCondition::LessThan` | `Filter::LessThan()` | field < value |
| `FilterCondition::LessThanOrEqual` | `Filter::LessThanOrEqualTo()` | field <= value |
| `FilterCondition::GreaterThan` | `Filter::GreaterThan()` | field > value |
| `FilterCondition::GreaterThanOrEqual` | `Filter::GreaterThanOrEqualTo()` | field >= value |
| `FilterCondition::ArrayContains` | `Filter::ArrayContains()` | Array membership |
| `FilterCondition::ArrayContainsAny` | `Filter::ArrayContainsAny()` | Array OR |
| `FilterCondition::In` | `Filter::In()` | Value in list |
| `FilterCondition::NotEqual` | `Filter::NotEqualTo()` | field != value |
| `FilterCondition::NotIn` | `Filter::NotIn()` | Value not in list |
| `FilterCondition::And` | `Filter::And()` | Conjunction (ALL) |
| `FilterCondition::Or` | `Filter::Or()` | Disjunction (ANY) |

### WriteBatch Class

#### ✅ Implemented (4 methods)

| Rust Method | C++ Reference | Notes |
|------------|---------------|-------|
| `set(doc_ref, data)` | `Set()` | Add write operation |
| `update(doc_ref, data)` | `Update()` | Add update operation |
| `delete(doc_ref)` | `Delete()` | Add delete operation |
| `commit()` | `Commit()` | Execute all operations |

### Transaction Class

#### ✅ Implemented (5 methods) - **FULLY INTEGRATED**

| Rust Method | C++ Reference | Notes |
|------------|---------------|-------|
| `get(doc_ref)` | `Get()` | Read in transaction |
| `set(doc_ref, data)` | `Set()` | Write in transaction |
| `update(doc_ref, data)` | `Update()` | Update in transaction |
| `delete(doc_ref)` | `Delete()` | Delete in transaction |
| `run_transaction()` | `RunTransaction()` | **NEW** - Fully integrated with Firestore |

**✅ Fully Implemented:** Transaction struct fully integrated via `Firestore::run_transaction()` with automatic retry logic (up to 5 attempts), read-before-write enforcement, and BeginTransaction/Commit/Rollback gRPC calls.

### DocumentSnapshot Class

#### ✅ Implemented (6 methods)

| Rust Method | C++ Reference | Notes |
|------------|---------------|-------|
| `id()` | `id()` | Document ID |
| `exists()` | `exists()` | Document existence |
| `data()` | `GetData()` | All fields as MapValue |
| `get(field)` | `Get()` | Single field value |
| `metadata` | `metadata()` | Snapshot metadata |
| `reference` | `reference()` | DocumentReference |

### Field Types

#### ✅ Implemented (10 types)

| Rust Type | C++ Reference | Notes |
|----------|---------------|-------|
| `Value` (protobuf) | `FieldValue` | Direct protobuf usage |
| `null_value` | `Null()` | Null type |
| `boolean_value` | `Boolean()` | bool |
| `integer_value` | `Integer()` | i64 |
| `double_value` | `Double()` | f64 |
| `timestamp_value` | `Timestamp()` | Timestamp struct |
| `string_value` | `String()` | String |
| `bytes_value` | `Blob()` | Binary data |
| `reference_value` | `Reference()` | DocumentReference |
| `geo_point_value` | `GeoPoint()` | GeoPoint struct |
| `array_value` | `Array()` | Vec<Value> |
| `map_value` | `Map()` | MapValue |

#### ❌ Missing (3 special types)

| C++ Type | Status | Priority |
|---------|--------|----------|
| `ServerTimestamp()` | 🔴 Not implemented | High - Auto timestamp |
| `ArrayUnion()` | 🔴 Not implemented | Medium - Array operations |
| `ArrayRemove()` | 🔴 Not implemented | Medium - Array operations |
| `Increment()` | 🔴 Not implemented | Medium - Numeric operations |
| `Delete()` | 🔴 Not implemented | Medium - Field removal |

---

## 🎯 Priority Implementation Roadmap

### 🔴 High Priority (Essential Features)

#### Authentication
1. **Password Reset Flow**
   - `ConfirmPasswordReset()`
   - `VerifyPasswordResetCode()`
2. **OAuth Provider Support**
   - `SignInWithProvider()` for OAuth flows
3. **Email Verification**
   - `SendEmailVerification()`

#### Firestore
1. **Server-side Field Values**
   - `ServerTimestamp()` for automatic timestamps
2. **Aggregation Queries**
   - `Count()` for efficient counting
3. ✅ **COMPLETED: Collection Group Queries**
   - `CollectionGroup()` for cross-collection queries
4. ✅ **COMPLETED: Transaction Support**
   - `Firestore::run_transaction()` integration

### 🟡 Medium Priority (Important Features)

#### Authentication
1. **Account Linking**
   - `LinkWithCredential()`
   - `UnlinkProvider()`
2. **Reauthentication**
   - `ReauthenticateWithCredential()`
3. **Phone Authentication**
   - `PhoneAuthProvider` implementation
4. ✅ **COMPLETED: Email Update Flow**
   - `SendEmailVerificationBeforeUpdatingEmail()`
5. **Persistence**
   - `SetPersistence()` for offline support

#### Firestore
1. **Network Control**
   - `EnableNetwork()` / `DisableNetwork()`
   - `WaitForPendingWrites()`
2. **Array/Numeric Operations**
   - `ArrayUnion()` / `ArrayRemove()`
   - `Increment()` / `Decrement()`
3. **Subcollections**
   - `DocumentReference::Collection()`
4. **Field Deletion**
   - `FieldValue::Delete()` for removing fields

### 🟢 Low Priority (Nice to Have)

#### Authentication
1. **Token Listeners**
   - `AddIdTokenListener()` / `RemoveIdTokenListener()`
2. **Email Link Auth**
   - `GetCredentialFromEmailLink()` / `IsEmailLink()`
3. **Legacy Providers**
   - `TwitterAuthProvider` / `YahooAuthProvider`

#### Firestore
1. **Cache Management**
   - `ClearPersistence()`
   - `Terminate()`
2. **Data Bundles**
   - `LoadBundle()` / `NamedQuery()`
3. **Advanced Aggregation**
   - Full `AggregateQuery` API

---

## 🏗️ Architecture Differences

### Rust-Specific Enhancements

1. **Stream-based Listeners** ✨
   - C++: Callback-based `AddSnapshotListener()`
   - Rust: Returns `Pin<Box<dyn Stream>>` for idiomatic async iteration
   - **Benefits:** Composable, automatic cleanup, async/await friendly

2. **Error Handling** ✨
   - C++: `Future<T>` with callbacks
   - Rust: `Result<T, FirebaseError>` with typed errors
   - **Benefits:** Compile-time error checking, exhaustive matching

3. **Type Safety** ✨
   - C++: `Variant` type for FieldValue
   - Rust: Direct protobuf `Value` with strong typing
   - **Benefits:** Zero-copy serialization, better performance

4. **Memory Management** ✨
   - C++: Shared pointers and manual lifecycle management
   - Rust: `Arc<T>` with automatic memory safety
   - **Benefits:** No memory leaks, no use-after-free

### C++ Features Not Needed in Rust

1. **Manual Resource Management**
   - C++: `Delete()`, `Terminate()`, explicit cleanup
   - Rust: RAII with Drop trait - automatic cleanup

2. **Thread Safety Primitives**
   - C++: Mutexes, condition variables
   - Rust: `RwLock`, `Mutex` with compile-time deadlock prevention

3. **Callback Registration**
   - C++: `AddListener()` / `RemoveListener()`
   - Rust: Streams with automatic unsubscribe on drop

---

## 📝 API Design Decisions

### Why Use Protobuf Value Directly?

**Decision:** Use `proto::google::firestore::v1::Value` instead of custom `FieldValue` enum

**Rationale:**
1. **Zero-copy:** No conversion between custom types and wire format
2. **Performance:** Direct serialization without intermediate representation
3. **Consistency:** Matches protobuf schema exactly
4. **Simplicity:** Less code to maintain, fewer bugs
5. **Compatibility:** Easy integration with other protobuf-based systems

**C++ Comparison:**
- C++ SDK uses `FieldValue` wrapper around Nanopb types
- Rust SDK uses Prost-generated types directly
- Both approaches are valid, Rust benefits from stronger type system

### Why Stream Instead of Callbacks?

**Decision:** Return `Stream` from listeners instead of callback registration

**Rationale:**
1. **Idiomatic Rust:** Async iterators are the standard pattern
2. **Composability:** Can use `StreamExt` combinators (map, filter, etc.)
3. **Automatic Cleanup:** Stream dropped = listener unregistered
4. **Error Propagation:** Errors are items in the stream
5. **Backpressure:** Natural flow control with `next().await`

**C++ Comparison:**
- C++ SDK: `AddSnapshotListener(callback)` returns `ListenerRegistration`
- Rust SDK: `listen()` returns `Stream<Item = Result<T>>`
- Both achieve the same goal with different idioms

---

## 🎓 Usage Patterns

### Authentication

```rust
// C++ style (conceptual)
// auth->SignInWithEmailAndPassword(email, password, callback);

// Rust style (actual)
let result = auth.sign_in_with_email_and_password(email, password).await?;
println!("Signed in: {}", result.user.uid);
```

### Firestore Queries

```rust
// C++ style (conceptual)
// collection->WhereEqualTo("age", 25)->OrderBy("name")->Limit(10)->Get(callback);

// Rust style (actual)
let snapshot = collection
    .where_field(FilterCondition::Equal("age".into(), Value::from(25)))
    .order_by("name", OrderDirection::Ascending)
    .limit(10)
    .get()
    .await?;
```

### Real-time Listeners

```rust
// C++ style (conceptual)
// ListenerRegistration reg = doc_ref->AddSnapshotListener(callback);
// // ... later ...
// reg.Remove();

// Rust style (actual)
let mut stream = doc_ref.listen().await?;
while let Some(result) = stream.next().await {
    match result {
        Ok(snapshot) => println!("Data: {:?}", snapshot.data()),
        Err(e) => eprintln!("Error: {}", e),
    }
}
// Stream auto-unregisters on drop
```

---

## 🚀 Conclusion

The Firebase Rust SDK has achieved **~48% API coverage** of the C++ SDK in its initial alpha release, focusing on the most commonly used features:

**Strong Coverage:**
- ✅ Core authentication flows (email/password, anonymous, OAuth)
- ✅ Firestore CRUD operations
- ✅ Real-time listeners with idiomatic Rust Streams
- ✅ Query operations (filters, ordering, pagination)
- ✅ Batch writes and transactions (structure complete)

**Key Gaps:**
- ❌ Password reset completion flow
- ❌ Email verification
- ❌ Account linking/unlinking
- ❌ Collection group queries
- ❌ Server-side field values (ServerTimestamp, etc.)
- ❌ Offline persistence (API exists but not implemented)

**Next Steps:**
1. Implement high-priority features (password reset, OAuth flows)
2. Add server-side field value transforms
3. Complete transaction integration
4. Implement offline persistence
5. Expand test coverage for edge cases

---

*This comparison is based on Firebase C++ SDK v11.x and Rust SDK v0.1.0-alpha.1*
