# Major Refactoring Plan: serde_json::Value → FieldValue

## Problem Statement

The current implementation uses `serde_json::Value` throughout the public API, which was a design mistake. This loses type safety and doesn't properly represent Firestore's rich type system including:
- Timestamps
- GeoPoints  
- DocumentReferences
- Blob data
- Sentinel values (Delete, ServerTimestamp, ArrayUnion, ArrayRemove, Increment)

## C++ SDK FieldValue Types

From `firestore/src/include/firebase/firestore/field_value.h`:

```cpp
enum class Type {
    // Real data types
    kNull,
    kBoolean,
    kInteger,
    kDouble,
    kTimestamp,
    kString,
    kBlob,
    kReference,
    kGeoPoint,
    kArray,
    kMap,
    
    // Sentinel types (write-only)
    kDelete,
    kServerTimestamp,
    kArrayUnion,
    kArrayRemove,
    kIncrementInteger,
    kIncrementDouble,
};
```

**Key Difference:** Firestore has specific types (Timestamp, GeoPoint, Blob, Reference) that `serde_json::Value` cannot represent.

## Rust FieldValue Design

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    // Data types
    Null,
    Boolean(bool),
    Integer(i64),
    Double(f64),
    Timestamp(Timestamp),
    String(String),
    Blob(Vec<u8>),
    Reference(DocumentReference),
    GeoPoint(GeoPoint),
    Array(Vec<FieldValue>),
    Map(HashMap<String, FieldValue>),
    
    // Sentinel values (write-only, never returned from reads)
    Delete,
    ServerTimestamp,
    ArrayUnion(Vec<FieldValue>),
    ArrayRemove(Vec<FieldValue>),
    Increment(IncrementValue),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncrementValue {
    Integer(i64),
    Double(f64),
}
```

## APIs Requiring Refactoring

### Firestore Module

#### Document Operations

**Current (WRONG):**
```rust
pub async fn set_document(&self, path: impl AsRef<str>, data: serde_json::Value) -> Result<(), FirebaseError>
pub async fn update_document(&self, path: impl AsRef<str>, data: serde_json::Value) -> Result<(), FirebaseError>
```

**Should be:**
```rust
pub async fn set_document(&self, path: impl AsRef<str>, data: HashMap<String, FieldValue>) -> Result<(), FirebaseError>
pub async fn update_document(&self, path: impl AsRef<str>, data: HashMap<String, FieldValue>) -> Result<(), FirebaseError>
```

**C++ Reference:** `firestore/src/common/document_reference.cc:87-124`

---

#### CollectionReference

**Current (WRONG):**
```rust
pub async fn add(&self, data: serde_json::Value) -> Result<DocumentReference, FirebaseError>
```

**Should be:**
```rust
pub async fn add(&self, data: HashMap<String, FieldValue>) -> Result<DocumentReference, FirebaseError>
```

**C++ Reference:** `firestore/src/common/collection_reference.cc:70`

---

#### DocumentSnapshot

**Current (WRONG):**
```rust
pub struct DocumentSnapshot {
    pub reference: DocumentReference,
    pub data: Option<serde_json::Value>,
    pub exists: bool,
}

impl DocumentSnapshot {
    pub fn get(&self, field: &str) -> Option<&serde_json::Value>
}
```

**Should be:**
```rust
pub struct DocumentSnapshot {
    pub reference: DocumentReference,
    pub data: Option<HashMap<String, FieldValue>>,
    pub exists: bool,
}

impl DocumentSnapshot {
    pub fn get(&self, field: &str) -> Option<&FieldValue>
}
```

**C++ Reference:** `firestore/src/common/document_snapshot.cc:87`

---

#### Query / FilterCondition

**Current (WRONG):**
```rust
pub enum FilterCondition {
    Equal(String, serde_json::Value),
    NotEqual(String, serde_json::Value),
    LessThan(String, serde_json::Value),
    // ... etc
    ArrayContains(String, serde_json::Value),
    ArrayContainsAny(String, Vec<serde_json::Value>),
    In(String, Vec<serde_json::Value>),
    NotIn(String, Vec<serde_json::Value>),
    And(Vec<FilterCondition>),
    Or(Vec<FilterCondition>),
}
```

**Should be:**
```rust
pub enum FilterCondition {
    Equal(String, FieldValue),
    NotEqual(String, FieldValue),
    LessThan(String, FieldValue),
    // ... etc
    ArrayContains(String, FieldValue),
    ArrayContainsAny(String, Vec<FieldValue>),
    In(String, Vec<FieldValue>),
    NotIn(String, Vec<FieldValue>),
    And(Vec<FilterCondition>),
    Or(Vec<FilterCondition>),
}
```

**C++ Reference:** `firestore/src/common/query.cc:142-380`

---

#### Query Pagination

**Current (WRONG):**
```rust
pub fn start_at(mut self, values: Vec<serde_json::Value>) -> Self
pub fn start_after(mut self, values: Vec<serde_json::Value>) -> Self
pub fn end_at(mut self, values: Vec<serde_json::Value>) -> Self
pub fn end_before(mut self, values: Vec<serde_json::Value>) -> Self
```

**Should be:**
```rust
pub fn start_at(mut self, values: Vec<FieldValue>) -> Self
pub fn start_after(mut self, values: Vec<FieldValue>) -> Self
pub fn end_at(mut self, values: Vec<FieldValue>) -> Self
pub fn end_before(mut self, values: Vec<FieldValue>) -> Self
```

**C++ Reference:** `firestore/src/common/query.cc:522-620`

---

#### WriteBatch

**Current (WRONG):**
```rust
pub struct WriteOperation {
    pub operation_type: String,
    pub path: String,
    pub data: Option<serde_json::Value>,
}

impl WriteBatch {
    pub fn set(&mut self, path: impl Into<String>, data: serde_json::Value) -> &mut Self
    pub fn update(&mut self, path: impl Into<String>, data: serde_json::Value) -> &mut Self
}
```

**Should be:**
```rust
pub struct WriteOperation {
    pub operation_type: String,
    pub path: String,
    pub data: Option<HashMap<String, FieldValue>>,
}

impl WriteBatch {
    pub fn set(&mut self, path: impl Into<String>, data: HashMap<String, FieldValue>) -> &mut Self
    pub fn update(&mut self, path: impl Into<String>, data: HashMap<String, FieldValue>) -> &mut Self
}
```

**C++ Reference:** `firestore/src/common/write_batch.cc:45`

---

#### Transaction

**Current (WRONG):**
```rust
impl Transaction {
    pub fn set(&mut self, path: impl Into<String>, data: serde_json::Value) -> &mut Self
    pub fn update(&mut self, path: impl Into<String>, data: serde_json::Value) -> &mut Self
}
```

**Should be:**
```rust
impl Transaction {
    pub fn set(&mut self, path: impl Into<String>, data: HashMap<String, FieldValue>) -> &mut Self
    pub fn update(&mut self, path: impl Into<String>, data: HashMap<String, FieldValue>) -> &mut Self
}
```

**C++ Reference:** `firestore/src/common/transaction.cc:87`

---

### Auth Module (OK - no changes needed)

The Auth module correctly uses simple types (String) for profile data. The `AdditionalUserInfo.profile` field uses `serde_json::Value` which is appropriate since it's arbitrary provider data.

```rust
pub struct AdditionalUserInfo {
    pub provider_id: String,
    pub is_new_user: bool,
    pub username: Option<String>,
    pub profile: Option<serde_json::Value>, // OK - arbitrary provider profile data
}
```

## Implementation Strategy

### Phase 1: Define FieldValue Type

1. Create proper `FieldValue` enum in `src/firestore/field_value.rs`
2. Implement conversion traits:
   - `From<bool>`, `From<i64>`, `From<f64>`, `From<String>` for convenience
   - `TryFrom<serde_json::Value>` for migration path
3. Implement serialization to Firestore REST API format
4. Implement deserialization from Firestore REST API responses

### Phase 2: Update Type Signatures

1. Update all function signatures to use `HashMap<String, FieldValue>` instead of `serde_json::Value`
2. Update `DocumentSnapshot.data` field type
3. Update `FilterCondition` variants
4. Update `WriteOperation.data` field type
5. Update pagination cursor types

### Phase 3: Update Implementation

1. Update `convert_value_to_firestore()` to work with `FieldValue`
2. Update `convert_firestore_fields()` to return `HashMap<String, FieldValue>`
3. Update query building logic
4. Update batch/transaction logic

### Phase 4: Update Tests

1. Convert all test data from `json!()` to `FieldValue` constructions
2. Add tests for special types (Timestamp, GeoPoint, Blob, Reference)
3. Add tests for sentinel values (Delete, ServerTimestamp, Increment, ArrayUnion/Remove)

### Phase 5: Update Documentation

1. Update all examples to use `FieldValue`
2. Document conversion helpers
3. Update API reference

## Migration Path for Users

Provide convenience macros/functions:

```rust
// Macro for simple cases
field_map! {
    "name" => "Alice",
    "age" => 30,
    "active" => true,
}

// Or builder pattern
FieldValueMap::new()
    .insert("name", "Alice")
    .insert("age", 30)
    .insert("active", true)
    .build()
```

## Benefits After Refactoring

1. **Type Safety**: Catch type errors at compile time
2. **Special Types**: Proper support for Timestamp, GeoPoint, Blob, DocumentReference
3. **Sentinel Values**: Explicit support for Delete, ServerTimestamp, Increment, etc.
4. **Better Errors**: Can validate field types before sending to server
5. **C++ Parity**: Matches C++ SDK's FieldValue design
6. **Documentation**: Self-documenting types instead of "use JSON"

## Files to Modify

```
src/firestore/
├── field_value.rs          # NEW - FieldValue enum and impls
├── types.rs                # UPDATE - DocumentSnapshot, FilterCondition, etc.
├── firestore.rs            # UPDATE - All document operations
└── query.rs                # UPDATE - Query building (if separate file)

tests/
├── firestore_tests.rs      # UPDATE - All tests
└── firestore_integration.rs # UPDATE - Integration tests

docs/
├── GETTING_STARTED.md      # UPDATE - Examples
└── API_REFERENCE.md        # UPDATE - Signatures
```

## Estimated Effort

- Phase 1 (FieldValue type): 2-3 days
- Phase 2 (Signatures): 1 day  
- Phase 3 (Implementation): 3-4 days
- Phase 4 (Tests): 2 days
- Phase 5 (Documentation): 1 day

**Total: ~10-12 days**

## Breaking Changes

⚠️ This is a **major breaking change**. All existing code using the SDK will need updates:

**Before:**
```rust
firestore.set_document("users/alice", json!({
    "name": "Alice",
    "age": 30
})).await?;
```

**After:**
```rust
use std::collections::HashMap;
use firebase_rust_sdk::firestore::FieldValue;

let mut data = HashMap::new();
data.insert("name".to_string(), FieldValue::String("Alice".to_string()));
data.insert("age".to_string(), FieldValue::Integer(30));
firestore.set_document("users/alice", data).await?;
```

**Or with helpers:**
```rust
firestore.set_document("users/alice", field_map! {
    "name" => "Alice",
    "age" => 30,
}).await?;
```

## Next Steps

1. Review and approve this plan
2. Create feature branch: `refactor/field-value`
3. Implement Phase 1 (FieldValue type definition)
4. Write comprehensive tests for FieldValue
5. Proceed with phases 2-5 systematically

---

**Decision Required:** Proceed with refactoring? This will temporarily break all existing code but result in a much better, type-safe API that properly matches the C++ SDK.
