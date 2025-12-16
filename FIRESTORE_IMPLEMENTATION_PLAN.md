# Firestore Implementation Plan

## Overview
Systematic implementation of Firestore API following C++ SDK header structure.
All implementations will reference corresponding C++ SDK files for accuracy.

## Reference Structure
- **C++ SDK Base**: `firebase-cpp-sdk/firestore/src/include/firebase/firestore/`
- **Core Implementation**: `firebase-ios-sdk/Firestore/core/src/`

## Implementation Status

### ✅ Phase 1: Core Infrastructure (COMPLETE)
- [x] Error types (`error.rs`) - C++ ref: `app.h:122`
- [x] Timestamp (`timestamp.rs`) - C++ ref: `timestamp.h:39`
- [x] FieldValue & MapValue (`field_value.rs`) - C++ ref: `field_value.h:97`
- [x] GeoPoint (`geo_point.rs`) - C++ ref: `geo_point.h:34`
- [x] Proto message integration via `field_value` crate

### ✅ Phase 2: Document & Collection (COMPLETE)
- [x] DocumentReference (`document_reference.rs`) - C++ ref: `document_reference.h:55`
- [x] DocumentSnapshot (`document_snapshot.rs`) - C++ ref: `document_snapshot.h:58`
- [x] CollectionReference (`collection_reference.rs`) - C++ ref: `collection_reference.h:44`
- [x] Firestore client (`firestore.rs`) - C++ ref: `firestore.h:106`
- [x] gRPC connection with TLS and authentication

### ✅ Phase 3: Query System (COMPLETE)
- [x] Query trait (`query.rs`) - C++ ref: `query.h:61`
- [x] QueryState for filters/orders/limits
- [x] QuerySnapshot (`query_snapshot.rs`) - C++ ref: `query_snapshot.h:55`
- [x] Filter operations (==, <, >, <=, >=, !=, array-contains, in, array-contains-any)
- [x] Order by (ascending/descending)
- [x] Limit operations
- [x] Cursor operations (startAt, startAfter, endAt, endBefore)

### ✅ Phase 4: Transactions (COMPLETE)
- [x] Transaction (`transaction.rs`) - C++ ref: `transaction.h:44`
- [x] Read-before-write enforcement
- [x] Transaction state tracking
- [x] Atomic operations (get, set, update, delete)

---

## Phase 5: WriteBatch ✅ (COMPLETED)

**Status**: Complete  
**Completion Date**: 2024-01-XX  
**Actual Effort**: ~3 hours

**C++ References:**
- `write_batch.h:40` - WriteBatch class
- `firestore/src/main/write_batch_main.cc` - Implementation

**Files Modified:**
- `src/firestore/write_batch.rs` - Updated to builder pattern
- `src/firestore/firestore.rs` - Updated batch() method

**Implemented Features:**
```rust
pub struct WriteBatch {
    firestore: Arc<FirestoreInner>,
    operations: Vec<WriteOperation>,
}

impl WriteBatch {
    pub fn new(firestore: Arc<FirestoreInner>) -> Self;
    pub fn set(mut self, path: impl Into<String>, data: MapValue) -> Self;
    pub fn update(mut self, path: impl Into<String>, data: MapValue) -> Self;
    pub fn delete(mut self, path: impl Into<String>) -> Self;
    pub async fn commit(self) -> Result<(), FirebaseError>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

**Design Decision:**
- ✅ Approved: Consuming self pattern (`mut self -> Self`) for builder-style chaining
- This allows better ownership semantics and prevents accidental reuse after commit

**Tests:** 1 unit test (operation structure validation), integration tests pending

**Complexity:** Medium (2-3 hours) - as estimated

---

## Phase 6: Listeners & Snapshots

**C++ References:**
- `listener_registration.h:34` - ListenerRegistration
- `snapshot_metadata.h:35` - SnapshotMetadata (partially done)
- `document_change.h:36` - DocumentChange

**Files to modify/create:**
- `src/firestore/listener.rs` (new)
- Update `document_snapshot.rs` with full SnapshotMetadata

**Key Features:**
```rust
pub struct ListenerRegistration {
    // Handle to cancel listener
}

impl ListenerRegistration {
    pub fn remove(self);
}

pub enum DocumentChangeType {
    Added,
    Modified,
    Removed,
}

pub struct DocumentChange {
    pub document: DocumentSnapshot,
    pub type_: DocumentChangeType,
    pub old_index: usize,
    pub new_index: usize,
}
```

**Design Questions:**
- How to handle async streams in Rust? Use `tokio::sync::mpsc` or `futures::stream`?
- Suggest: `async fn add_snapshot_listener<F>(callback: F) -> ListenerRegistration where F: Fn(QuerySnapshot) + Send + 'static`

**Estimated Complexity:** High (4-6 hours) - involves async streams and lifecycle management

---

## Phase 7: Settings & Configuration

**C++ References:**
- `settings.h:43` - Settings class
- `firestore.h:106` - Firestore::set_settings()

**Files to modify:**
- Update `src/firestore/firestore.rs`
- Create `src/firestore/settings.rs`

**Key Features:**
```rust
pub struct Settings {
    pub host: String,
    pub ssl_enabled: bool,
    pub persistence_enabled: bool,
    pub cache_size_bytes: i64,
}

impl Settings {
    pub fn default() -> Self;
}
```

**Design Questions:**
- Should persistence be implemented? (C++ has local cache via LevelDB)
- Suggest: Start with in-memory only, add persistence later as separate phase

**Estimated Complexity:** Low (1-2 hours)

---

## Phase 8: Aggregation Queries

**C++ References:**
- `aggregate_query.h:36` - AggregateQuery
- `aggregate_query_snapshot.h:35` - AggregateQuerySnapshot

**Files to create:**
- `src/firestore/aggregate_query.rs`
- `src/firestore/aggregate_query_snapshot.rs`

**Key Features:**
```rust
pub struct AggregateQuery {
    // Query with aggregations
}

impl AggregateQuery {
    pub fn count() -> AggregateField;
    pub fn sum(field: &str) -> AggregateField;
    pub fn average(field: &str) -> AggregateField;
    pub async fn get(&self) -> Result<AggregateQuerySnapshot, FirebaseError>;
}

pub struct AggregateQuerySnapshot {
    // Aggregation results
}
```

**Design Questions:**
- Which aggregations to implement first? (C++ has count, sum, average)
- Suggest: Start with count() only, most commonly used

**Estimated Complexity:** Medium (3-4 hours)

---

## Phase 9: Load Bundles (LOW PRIORITY)

**C++ References:**
- `load_bundle_task_progress.h:35` - LoadBundleTaskProgress

**Files to create:**
- `src/firestore/bundle.rs`

**Estimated Complexity:** Medium-High (3-5 hours)

---

## Phase 10: Offline Persistence (FUTURE)

**C++ References:**
- Uses LevelDB for local storage
- `firestore/src/main/settings_main.cc` - persistence settings

**Design Questions:**
- Use sled, redb, or rocksdb for Rust?
- How to handle cache eviction policies?
- Should we implement this at all for initial release?

**Estimated Complexity:** Very High (8-12 hours) - major feature

---

## Phase 11: Testing & Integration

**Files to create:**
- `tests/firestore_integration_tests.rs`
- `examples/firestore_example.rs`

**Test Coverage Goals:**
- [ ] Document CRUD operations
- [ ] Query filtering and ordering
- [ ] Transactions (read-modify-write patterns)
- [ ] WriteBatch operations
- [ ] Listeners and real-time updates
- [ ] Error handling and retries
- [ ] Concurrent operations

**Estimated Complexity:** High (6-8 hours)

---

## Implementation Guidelines

### 1. C++ Reference Pattern
For each new feature:
1. Read C++ header file in `firebase-cpp-sdk/firestore/src/include/firebase/firestore/`
2. Check implementation in `firebase-cpp-sdk/firestore/src/main/` or `firebase-ios-sdk/Firestore/core/src/`
3. Document C++ reference in Rust file header comments
4. Match method signatures and behavior

### 2. Error Handling
- Use error-first control flow (check errors before success)
- Map gRPC status codes to `FirestoreError` variants
- Provide clear error messages with context

### 3. Async Patterns
- All network operations are `async`
- Use `tokio::spawn` for background tasks
- Use `Arc` for shared state, avoid `RwLock` where possible

### 4. Testing
- Unit tests for each module
- Integration tests require Firebase project setup
- Mock gRPC responses for offline testing

### 5. Documentation
- Document all public APIs with examples
- Reference C++ SDK line numbers in comments
- Explain differences from C++ where applicable

---

## Decision Points (REQUIRE USER APPROVAL)

### 1. WriteBatch API Design
**Question:** Should WriteBatch use builder pattern (`&mut self`) or functional pattern (consume `self`)?
**Recommendation:** Builder pattern with `&mut self` returning `&mut Self` for chaining
**Reasoning:** Matches C++ SDK, allows flexible operation ordering

### 2. Snapshot Listeners
**Question:** Use callback-based or Stream-based API for listeners?
**Options:**
- A) Callback: `add_listener(|snapshot| { ... }) -> ListenerRegistration`
- B) Stream: `add_listener() -> (impl Stream<Item=QuerySnapshot>, ListenerRegistration)`
**Recommendation:** Option A (callbacks) initially, matches C++ SDK
**Reasoning:** Easier to implement, matches user expectations from other SDKs

### 3. Persistence Layer
**Question:** Should we implement offline persistence in initial release?
**Options:**
- A) In-memory cache only (simple, reliable)
- B) Optional persistence via feature flag
- C) Always-on persistence (matches C++)
**Recommendation:** Option A initially, Option B in Phase 10
**Reasoning:** Reduce complexity for initial stable release

### 4. Field Value Types
**Question:** Current implementation uses proto types directly. Should we wrap them?
**Current:** `MapValue { fields: HashMap<String, proto::Value> }`
**Alternative:** Custom `FieldValue` enum wrapping proto types
**Recommendation:** Keep current proto-based approach
**Reasoning:** Already working, avoids double-conversion overhead

### 5. Query Trait vs Struct
**Question:** Keep Query as trait or convert to concrete struct?
**Current:** `pub trait Query` implemented by `CollectionReference`
**Alternative:** `pub struct Query { state: QueryState }` with CollectionReference containing Query
**Recommendation:** Keep current trait approach
**Reasoning:** Validated against C++ SDK, only CollectionReference implements Query

---

## Priority Ranking

### Must Have (v0.1.0)
1. ✅ Core types (Timestamp, FieldValue, GeoPoint)
2. ✅ Document operations (CRUD)
3. ✅ Query system (filters, ordering, limits)
4. ✅ Transactions
5. WriteBatch (Phase 5)
6. Basic error handling and retries

### Should Have (v0.2.0)
7. Snapshot listeners (Phase 6)
8. Settings configuration (Phase 7)
9. Aggregation queries (Phase 8)
10. Comprehensive integration tests

### Nice to Have (v0.3.0+)
11. Load bundles (Phase 9)
12. Offline persistence (Phase 10)
13. Performance optimizations
14. Connection pooling

---

## Next Steps

1. **Immediate:** Implement WriteBatch (Phase 5)
   - Est: 2-3 hours
   - Blocking: None
   - Required for: Complete write operations

2. **After WriteBatch:** Implement snapshot listeners (Phase 6)
   - Est: 4-6 hours
   - Blocking: Need async stream design decision
   - Required for: Real-time updates

3. **Quick Win:** Add Settings (Phase 7)
   - Est: 1-2 hours
   - Blocking: None
   - Required for: Production configuration

4. **Feature Complete:** Aggregation queries (Phase 8)
   - Est: 3-4 hours
   - Blocking: None
   - Required for: Analytics use cases

---

## Questions for User

Before proceeding with Phase 5 (WriteBatch), please confirm:

1. **WriteBatch API:** Approve builder pattern with `&mut self` returning `&mut Self`?
2. **Listener API:** Approve callback-based approach for Phase 6?
3. **Persistence:** Defer offline persistence to Phase 10 (post v0.1.0)?
4. **Priority Order:** Agree with Phase 5 → Phase 6 → Phase 7 → Phase 8 sequence?
5. **Testing Strategy:** Integration tests with real Firebase project or mocks?

Please respond with any concerns or alternative preferences before I proceed.
