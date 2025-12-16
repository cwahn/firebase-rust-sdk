# Query Architecture Refactor Plan

## Status: In Progress
Created: 2024-12-16

## Overview
Refactor Firestore types to match C++ SDK architecture where `Query` is an abstract base class and `CollectionReference` inherits from it. Also split the large `types.rs` file into separate modules following C++ namespace structure.

## C++ SDK Architecture Review

### Query Class (Abstract Base)
**File**: `firebase-cpp-sdk/firestore/src/include/firebase/firestore/query.h`

```cpp
class Query {
 public:
  // Query refinement methods (return new Query)
  virtual Query Where(const Filter& filter) const;
  virtual Query WhereEqualTo(const std::string& field, const FieldValue& value) const;
  virtual Query WhereNotEqualTo(const std::string& field, const FieldValue& value) const;
  virtual Query WhereLessThan(const std::string& field, const FieldValue& value) const;
  virtual Query WhereLessThanOrEqualTo(const std::string& field, const FieldValue& value) const;
  virtual Query WhereGreaterThan(const std::string& field, const FieldValue& value) const;
  virtual Query WhereGreaterThanOrEqualTo(const std::string& field, const FieldValue& value) const;
  virtual Query WhereArrayContains(const std::string& field, const FieldValue& value) const;
  virtual Query WhereArrayContainsAny(const std::string& field, const std::vector<FieldValue>& values) const;
  virtual Query WhereIn(const std::string& field, const std::vector<FieldValue>& values) const;
  virtual Query WhereNotIn(const std::string& field, const std::vector<FieldValue>& values) const;
  
  virtual Query OrderBy(const std::string& field, Direction direction = kAscending) const;
  virtual Query Limit(int32_t limit) const;
  virtual Query LimitToLast(int32_t limit) const;
  virtual Query StartAt(const DocumentSnapshot& snapshot) const;
  virtual Query StartAfter(const DocumentSnapshot& snapshot) const;
  virtual Query EndBefore(const DocumentSnapshot& snapshot) const;
  virtual Query EndAt(const DocumentSnapshot& snapshot) const;
  
  // Execution methods
  virtual Future<QuerySnapshot> Get(Source source = Source::kDefault) const;
  virtual ListenerRegistration AddSnapshotListener(...);
  
  // Aggregate queries
  virtual AggregateQuery Count() const;
};
```

### CollectionReference (Inherits Query)
**File**: `firebase-cpp-sdk/firestore/src/include/firebase/firestore/collection_reference.h`

```cpp
class CollectionReference : public Query {
 public:
  // Collection-specific methods
  std::string id() const;
  std::string path() const;
  DocumentReference Parent() const;
  DocumentReference Document() const;
  DocumentReference Document(const std::string& document_path) const;
  Future<DocumentReference> Add(const MapFieldValue& data);
};
```

### Firestore::CollectionGroup()
**File**: `firebase-cpp-sdk/firestore/src/include/firebase/firestore.h:268`

```cpp
virtual Query CollectionGroup(const char* collection_id) const;
```

**Returns `Query`, not `CollectionReference`** - this is the key architectural point.

---

## Tasks

### ✅ Phase 1: File Structure Refactoring (Split types.rs)

Following C++ namespace structure:
- `firestore/src/include/firebase/firestore/field_value.h` → `field_value.rs`
- `firestore/src/include/firebase/firestore/timestamp.h` → `timestamp.rs`
- `firestore/src/include/firebase/firestore/geo_point.h` → `geo_point.rs`
- `firestore/src/include/firebase/firestore/document_reference.h` → `document_reference.rs`
- `firestore/src/include/firebase/firestore/document_snapshot.h` → `document_snapshot.rs`
- `firestore/src/include/firebase/firestore/collection_reference.h` → `collection_reference.rs`
- `firestore/src/include/firebase/firestore/query.h` → `query.rs` (NEW)
- `firestore/src/include/firebase/firestore/query_snapshot.h` → `query_snapshot.rs`
- `firestore/src/include/firebase/firestore/write_batch.h` → `write_batch.rs`
- `firestore/src/include/firebase/firestore/settings.h` → `settings.rs`
- `firestore/src/include/firebase/firestore/listener_registration.h` → `listener_registration.rs`

**Sub-tasks:**
- [ ] 1.1: Create `src/firestore/timestamp.rs` - Extract Timestamp type
- [ ] 1.2: Create `src/firestore/geo_point.rs` - Extract GeoPoint type
- [ ] 1.3: Create `src/firestore/settings.rs` - Extract Settings and Source types
- [ ] 1.4: Create `src/firestore/document_snapshot.rs` - Extract DocumentSnapshot and SnapshotMetadata
- [ ] 1.5: Create `src/firestore/query_snapshot.rs` - Extract QuerySnapshot and DocumentChange
- [ ] 1.6: Create `src/firestore/listener_registration.rs` - Extract ListenerRegistration
- [ ] 1.7: Create `src/firestore/write_batch.rs` - Extract WriteBatch and WriteOperation
- [ ] 1.8: Create `src/firestore/document_reference.rs` - Extract DocumentReference
- [ ] 1.9: Update `src/firestore/mod.rs` with new module exports
- [ ] 1.10: Update all imports across codebase
- [ ] 1.11: Verify all tests pass after refactor
- [ ] 1.12: Commit: "refactor(firestore): Split types.rs into separate modules following C++ structure"

### ⬜ Phase 2: Query Trait Implementation

**Sub-tasks:**
- [ ] 2.1: Create `src/firestore/query.rs` with Query trait
  - Review C++ Query class methods (lines 142-500 of query.h)
  - Define trait with all query refinement methods
  - Add Direction enum (Ascending/Descending)
  - Add query execution methods (get, listen)
- [ ] 2.2: Implement QueryBuilder struct
  - Internal struct to hold query state (filters, orders, limits)
  - Implements Query trait
  - Clone support for immutable query chaining
- [ ] 2.3: Update CollectionReference to implement Query trait
  - Add collection-specific methods (id, path, document, add)
  - Delegate query methods to internal QueryBuilder
- [ ] 2.4: Update Firestore::collection_group() return type
  - Change from `CollectionReference` to `Box<dyn Query>`
  - Add integration test for collection_group queries
- [ ] 2.5: Write Query trait tests
  - Test query chaining (where, order_by, limit)
  - Test immutability (original query unchanged)
  - Test all filter operators
- [ ] 2.6: Commit: "feat(firestore): Implement Query trait matching C++ abstract class"

### ⬜ Phase 3: Transaction Implementation & Tests

**C++ Reference**: `firebase-cpp-sdk/firestore/src/include/firebase/firestore/transaction.h`

**Sub-tasks:**
- [ ] 3.1: Review C++ Transaction implementation
  - Read transaction.h (complete interface)
  - Read firestore/src/common/transaction.cc (Get implementation)
  - Understand BeginTransaction/Commit gRPC flow
- [ ] 3.2: Implement Transaction::get() with gRPC
  - Replace REST API stub with gRPC BatchGetDocuments
  - Store transaction ID from first read
  - Add read tracking for validation
- [ ] 3.3: Implement Transaction write operations
  - Ensure set/update/delete accumulate operations
  - Validate write-after-read ordering
- [ ] 3.4: Implement Firestore::run_transaction() retry logic
  - BeginTransaction gRPC call
  - Execute user function with transaction
  - Commit with retry on conflicts (up to 5 attempts)
  - Rollback on errors
- [ ] 3.5: Write Transaction unit tests (10+ tests)
  - Test get() retrieves document
  - Test set/update/delete operations
  - Test read-before-write validation
  - Test transaction ID handling
  - Test empty transaction (no operations)
  - Test write-only transaction
  - Test read-only transaction
  - Test multiple reads
  - Test error handling
- [ ] 3.6: Write run_transaction() integration tests (8+ tests)
  - Test successful transaction commit
  - Test retry on conflict
  - Test max retry limit
  - Test rollback on error
  - Test read-modify-write pattern
  - Test concurrent transactions
  - Test transaction isolation
  - Test nested field updates in transaction
- [ ] 3.7: Commit: "feat(firestore): Complete Transaction implementation with tests"

### ⬜ Phase 4: WriteBatch Tests

**Sub-tasks:**
- [ ] 4.1: Write WriteBatch unit tests (8+ tests)
  - Test batch creation
  - Test set operation
  - Test update operation
  - Test delete operation
  - Test multiple operations
  - Test empty batch (should error on commit)
  - Test batch with all operation types
  - Test batch immutability
- [ ] 4.2: Write WriteBatch integration tests (5+ tests)
  - Test successful batch commit
  - Test batch atomicity (all or nothing)
  - Test batch size limits
  - Test batch with document references
  - Test batch error handling
- [ ] 4.3: Commit: "test(firestore): Add comprehensive WriteBatch tests"

### ⬜ Phase 5: DocumentReference Tests

**Sub-tasks:**
- [ ] 5.1: Write DocumentReference unit tests (10+ tests)
  - Test document path parsing
  - Test id() extraction
  - Test parent_path() extraction
  - Test full_path() generation
  - Test path validation
  - Test nested collection paths
  - Test subcollection access
  - Test equality comparison
  - Test clone behavior
  - Test debug formatting
- [ ] 5.2: Write DocumentReference integration tests (8+ tests)
  - Test set() operation
  - Test update() operation
  - Test delete() operation
  - Test get() operation
  - Test non-existent document
  - Test document with nested fields
  - Test document with arrays
  - Test document with timestamps
- [ ] 5.3: Commit: "test(firestore): Add comprehensive DocumentReference tests"

### ⬜ Phase 6: CollectionReference Tests

**Sub-tasks:**
- [ ] 6.1: Write CollectionReference unit tests (8+ tests)
  - Test collection path parsing
  - Test id() extraction
  - Test document() creation
  - Test path validation
  - Test nested collection paths
  - Test root collection
  - Test subcollection access
  - Test query inheritance (after Query trait)
- [ ] 6.2: Write CollectionReference integration tests (6+ tests)
  - Test add() with auto-generated ID
  - Test document() reference creation
  - Test query operations (where, order_by, limit)
  - Test collection_group queries
  - Test listener attachment
  - Test pagination
- [ ] 6.3: Commit: "test(firestore): Add comprehensive CollectionReference tests"

### ⬜ Phase 7: Query Tests

**Sub-tasks:**
- [ ] 7.1: Write Query trait unit tests (15+ tests)
  - Test where_equal_to filter
  - Test where_not_equal_to filter
  - Test where_less_than filter
  - Test where_less_than_or_equal filter
  - Test where_greater_than filter
  - Test where_greater_than_or_equal filter
  - Test where_array_contains filter
  - Test where_array_contains_any filter
  - Test where_in filter
  - Test where_not_in filter
  - Test compound filters (And/Or)
  - Test order_by ascending
  - Test order_by descending
  - Test limit
  - Test query chaining immutability
- [ ] 7.2: Write Query integration tests (10+ tests)
  - Test simple query execution
  - Test multiple filters
  - Test ordering with filtering
  - Test limit with ordering
  - Test pagination (startAt, endAt)
  - Test collection_group queries
  - Test query with listener
  - Test query with source options (cache/server)
  - Test query with non-existent data
  - Test query performance with indices
- [ ] 7.3: Commit: "test(firestore): Add comprehensive Query tests"

### ⬜ Phase 8: Integration & Documentation

**Sub-tasks:**
- [ ] 8.1: Update API_COMPARISON.md
  - Mark Query trait as implemented
  - Update CollectionReference architecture notes
  - Update Firestore::collection_group() return type
- [ ] 8.2: Update README.md with Query examples
  - Show query chaining
  - Show collection_group usage
  - Show transaction patterns
- [ ] 8.3: Add rustdoc examples for all Query methods
- [ ] 8.4: Run full test suite
- [ ] 8.5: Run clippy and fix warnings
- [ ] 8.6: Commit: "docs: Update documentation for Query trait architecture"

---

## Architecture Notes

### Query Trait Design (Rust)

```rust
pub trait Query: Clone {
    // Query refinement (returns Self for chaining)
    fn where_equal_to(&self, field: &str, value: Value) -> Self;
    fn where_not_equal_to(&self, field: &str, value: Value) -> Self;
    fn where_less_than(&self, field: &str, value: Value) -> Self;
    fn where_less_than_or_equal(&self, field: &str, value: Value) -> Self;
    fn where_greater_than(&self, field: &str, value: Value) -> Self;
    fn where_greater_than_or_equal(&self, field: &str, value: Value) -> Self;
    fn where_array_contains(&self, field: &str, value: Value) -> Self;
    fn where_array_contains_any(&self, field: &str, values: Vec<Value>) -> Self;
    fn where_in(&self, field: &str, values: Vec<Value>) -> Self;
    fn where_not_in(&self, field: &str, values: Vec<Value>) -> Self;
    
    fn order_by(&self, field: &str, direction: OrderDirection) -> Self;
    fn limit(&self, count: i64) -> Self;
    fn limit_to_last(&self, count: i64) -> Self;
    
    // Execution
    fn get(&self) -> impl Future<Output = Result<QuerySnapshot, FirebaseError>>;
    fn listen(&self, callback: Box<dyn Fn(QuerySnapshot)>) -> ListenerRegistration;
}

// Internal implementation
struct QueryBuilder {
    firestore: Arc<FirestoreInner>,
    collection_path: Option<String>,  // None for collection_group
    collection_id: Option<String>,     // For collection_group
    filters: Vec<FilterCondition>,
    orders: Vec<(String, OrderDirection)>,
    limit_count: Option<i64>,
    // ... cursor fields
}

impl Query for QueryBuilder { /* ... */ }

// CollectionReference implements Query by delegating
impl Query for CollectionReference {
    fn where_equal_to(&self, field: &str, value: Value) -> Self {
        let mut new_ref = self.clone();
        // Modify internal query builder
        new_ref
    }
    // ...
}
```

### Key Design Decisions

1. **Query as Trait**: Matches C++ abstract base class pattern
2. **Immutable Chaining**: Each query method returns new instance (like C++)
3. **CollectionReference Inherits Query**: Via trait implementation
4. **collection_group() Returns Query**: Not CollectionReference
5. **QueryBuilder Internal**: Concrete implementation, not exposed publicly

---

## Testing Strategy

### Test Coverage Goals
- **Unit Tests**: 60+ tests across all types
- **Integration Tests**: 40+ tests with real Firestore operations
- **Test Categories**:
  - Query operations (filtering, ordering, limiting)
  - Transaction operations (read, write, retry)
  - Batch operations (atomic writes)
  - Document operations (CRUD)
  - Collection operations (add, query)
  - Listener operations (real-time updates)

### Test Organization
```
tests/
  firestore/
    query_tests.rs          (15 unit + 10 integration)
    transaction_tests.rs    (10 unit + 8 integration)
    write_batch_tests.rs    (8 unit + 5 integration)
    document_ref_tests.rs   (10 unit + 8 integration)
    collection_ref_tests.rs (8 unit + 6 integration)
```

---

## Git Commit Strategy

Each phase will have focused commits:
1. File refactoring commit (types.rs split)
2. Query trait implementation commit
3. Transaction implementation commit
4. Test commits (one per major feature)
5. Documentation update commit

All commits will be made to `main` branch only after user approval.

---

## Success Criteria

- [ ] types.rs split into 10+ focused modules
- [ ] Query trait implemented and working
- [ ] CollectionReference implements Query trait
- [ ] Firestore::collection_group() returns Query
- [ ] Transaction fully implemented with gRPC
- [ ] 100+ total tests passing
- [ ] All tests documented with C++ SDK references
- [ ] API_COMPARISON.md updated
- [ ] No clippy warnings
- [ ] All code compiles without warnings

---

## Notes

- User must approve before any changes to main branch
- Each task completion requires commit
- Test coverage is mandatory for all features
- Follow C++ SDK architecture strictly
- Document all C++ references in code comments
