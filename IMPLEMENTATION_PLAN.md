# High-Priority API Implementation Plan

## Overview
Implementing high-priority missing APIs based on C++ SDK investigation and API_COMPARISON.md analysis.

## APIs to Implement

### 1. Authentication APIs

#### 1.1 SendEmailVerification
**C++ Reference**: `firebase-cpp-sdk/auth/src/desktop/rpcs/get_oob_confirmation_code_request.cc`
- **Endpoint**: `getOobConfirmationCode`
- **Request Type**: `VERIFY_EMAIL`
- **Method**: POST to `https://identitytoolkit.googleapis.com/v1/accounts:sendOobCode?key={API_KEY}`
- **Request Body**:
  ```json
  {
    "requestType": "VERIFY_EMAIL",
    "idToken": "user_id_token"
  }
  ```
- **Optional Header**: `X-Firebase-Locale` for language_code
- **Implementation Location**: `src/auth/types.rs` - add method to `User` struct
- **Signature**: `pub async fn send_email_verification(&self) -> Result<(), FirebaseError>`

#### 1.2 SendEmailVerificationBeforeUpdatingEmail
**C++ Reference**: Same file as above
- **Endpoint**: `getOobConfirmationCode`
- **Request Type**: `VERIFY_AND_CHANGE_EMAIL`
- **Request Body**:
  ```json
  {
    "requestType": "VERIFY_AND_CHANGE_EMAIL",
    "idToken": "user_id_token",
    "newEmail": "new@example.com"
  }
  ```
- **Implementation Location**: `src/auth/types.rs` - add method to `User` struct
- **Signature**: `pub async fn send_email_verification_before_updating_email(&self, new_email: &str) -> Result<(), FirebaseError>`

#### 1.3 PhoneAuthProvider (DEFERRED - Medium Priority)
**Status**: Complex - requires SMS service integration
- Requires phone verification infrastructure
- Need to investigate Firebase phone auth RPC details
- Consider implementing in Phase 2

#### 1.4 UpdatePhoneNumberCredential (DEFERRED - Medium Priority)
**Status**: Depends on PhoneAuthProvider
- Implement after PhoneAuthProvider is complete
- Uses `setAccountInfo` RPC with phone number credential

### 2. Firestore APIs

#### 2.1 CollectionGroup
**C++ Reference**: `firebase-cpp-sdk/firestore/src/include/firebase/firestore.h:268`
- **Method**: `Query CollectionGroup(const char* collection_id)`
- **Description**: Returns a Query that includes all documents in the database contained in a collection or subcollection with the given collection_id
- **gRPC API**: Uses `StructuredQuery` with `allDescendants: true`
- **Implementation Location**: `src/firestore/firestore.rs`
- **Signature**: `pub fn collection_group(&self, collection_id: &str) -> Result<CollectionGroupQuery, FirebaseError>`
- **Key Difference**: Sets `from.allDescendants = true` in StructuredQuery

#### 2.2 RunTransaction
**C++ Reference**: `firebase-cpp-sdk/firestore/src/include/firebase/firestore.h:310`
- **Method**: `Future<void> RunTransaction(std::function<Error(Transaction&, std::string&)> update)`
- **gRPC Calls**:
  1. `BeginTransaction` - starts transaction, returns transaction ID
  2. User callback executes with `Transaction` object
  3. `Commit` - commits changes with transaction ID
  4. `Rollback` - on error or explicit rollback
- **Retry Logic**: Up to 5 attempts (default) if transaction fails due to conflicts
- **Implementation Location**: `src/firestore/firestore.rs` + new `src/firestore/transaction.rs`
- **Signature**: 
  ```rust
  pub async fn run_transaction<F, R>(&self, update: F) -> Result<R, FirebaseError>
  where
      F: FnOnce(&Transaction) -> Result<R, FirebaseError>,
  ```
- **Transaction Methods**:
  - `get(&self, doc_ref: &DocumentReference) -> Result<Option<Document>, FirebaseError>`
  - `set(&mut self, doc_ref: &DocumentReference, data: Value) -> Result<(), FirebaseError>`
  - `update(&mut self, doc_ref: &DocumentReference, data: Value) -> Result<(), FirebaseError>`
  - `delete(&mut self, doc_ref: &DocumentReference) -> Result<(), FirebaseError>`

## Implementation Order

### Phase 1: Email Verification (Easier, Immediate Value)
1. ✅ Investigate C++ implementation
2. Implement `send_email_verification()` 
3. Implement `send_email_verification_before_updating_email()`
4. Write unit tests
5. Write integration tests
6. Update API_COMPARISON.md

### Phase 2: CollectionGroup Queries (Medium Complexity)
1. ✅ Investigate C++ implementation
2. Create `CollectionGroupQuery` type (or reuse `Query` with flag)
3. Implement `collection_group()` method in `Firestore`
4. Modify query building to handle `allDescendants` flag
5. Write integration tests
6. Update API_COMPARISON.md

### Phase 3: Transactions (Complex, High Value)
1. ✅ Investigate C++ implementation and gRPC calls
2. Create `Transaction` struct in new file
3. Implement transaction state management (read set, write buffer)
4. Implement `run_transaction()` with retry logic
5. Implement `Transaction` methods (get/set/update/delete)
6. Write comprehensive integration tests (conflicts, retries, rollback)
7. Update API_COMPARISON.md

### Phase 4: Phone Auth (Deferred to Future)
- Requires additional research
- May need external SMS service integration
- Lower priority for alpha release

## Technical Details

### Authentication REST API Base URL
```
https://identitytoolkit.googleapis.com/v1/accounts
```

### Firestore gRPC Protos
Already imported via `firestore_proto`:
- `BeginTransactionRequest/Response`
- `CommitRequest/Response` 
- `RollbackRequest`
- `StructuredQuery` (for CollectionGroup)

### Error Handling
- Map Firebase REST/gRPC errors to `FirebaseError` enum
- Handle authentication errors (expired token, invalid token)
- Handle Firestore errors (transaction conflicts, precondition failures)

### Testing Strategy
1. Unit tests for input validation
2. Integration tests with live Firebase project
3. Test error cases (invalid email, missing auth, etc.)
4. Test edge cases (transaction retries, collection group across subcollections)

## Success Criteria
- [ ] All new methods compile without errors
- [ ] Integration tests pass with live Firebase
- [ ] API_COMPARISON.md updated with new coverage percentages
- [ ] Code follows existing patterns (error-first, Arc usage, etc.)
- [ ] Documentation comments added for all public APIs
- [ ] Commit references C++ SDK implementation locations

## Estimated Coverage Increase
- **Current**: Auth 49%, Firestore 47%
- **After Phase 1**: Auth 54% (+2 methods)
- **After Phase 2**: Firestore 49% (+1 method)
- **After Phase 3**: Firestore 51% (+1 method + Transaction type)
- **Target**: Auth 54%, Firestore 51%, Overall ~52%

## Notes
- Phone auth deferred due to complexity and SMS infrastructure requirements
- Focus on high-value, achievable APIs first
- Transaction retry logic critical for correctness
- CollectionGroup must properly set `allDescendants` flag in query
