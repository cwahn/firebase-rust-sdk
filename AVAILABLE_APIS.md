# Firebase C++ SDK - Available APIs for Rust Translation

This document lists the public APIs from Firebase C++ SDK's Auth and Firestore modules that are available for translation to Rust.

## Auth Module APIs

### Core Classes

#### `firebase::auth::Auth`
Main authentication manager class.

**Static Methods:**
- `GetAuth(App*, InitResult*)` - Get Auth instance for an app

**Instance Methods:**
- `current_user()` - Get currently signed-in user
- `language_code()` - Get current language code
- `set_language_code(const char*)` - Set language code
- `UseAppLanguage()` - Use device's default language

**Sign-in Methods:**
- `SignInAnonymously()` - Sign in anonymously
- `SignInWithEmailAndPassword(email, password)` - Email/password sign-in
- `SignInWithCustomToken(token)` - Custom token sign-in
- `SignInWithCredential(credential)` - Sign in with credential
- `SignInWithProvider(provider)` - Sign in with federated provider
- `SignInAndRetrieveDataWithCredential(credential)` - Sign in with additional user info
- `SignOut()` - Sign out current user

**User Management:**
- `CreateUserWithEmailAndPassword(email, password)` - Create new user
- `SendPasswordResetEmail(email)` - Send password reset email
- `FetchProvidersForEmail(email)` - Get available providers for email

**Listeners:**
- `AddAuthStateListener(listener)` - Listen to auth state changes
- `RemoveAuthStateListener(listener)` - Remove auth state listener
- `AddIdTokenListener(listener)` - Listen to ID token changes
- `RemoveIdTokenListener(listener)` - Remove ID token listener

**Testing:**
- `UseEmulator(host, port)` - Connect to auth emulator

**Result Getters (LastResult methods):**
- All async methods have corresponding `*LastResult()` methods

---

#### `firebase::auth::User`
Represents a user account.

**Properties:**
- `uid()` - Unique user ID
- `email()` - User's email
- `display_name()` - Display name
- `photo_url()` - Photo URL
- `provider_id()` - Provider ID
- `phone_number()` - Phone number
- `is_valid()` - Check if user is valid
- `is_email_verified()` - Check if email is verified
- `is_anonymous()` - Check if anonymous user
- `metadata()` - Get user metadata
- `provider_data()` - Get provider data

**Token Management:**
- `GetToken(force_refresh)` - Get ID token
- `GetTokenThreadSafe(force_refresh)` - Thread-safe token getter

**Profile Management:**
- `UpdateUserProfile(profile)` - Update display name/photo
- `UpdatePassword(password)` - Change password
- `UpdateEmail(email)` - Change email
- `UpdatePhoneNumberCredential(credential)` - Update phone number

**Email Verification:**
- `SendEmailVerification()` - Send verification email
- `SendEmailVerificationBeforeUpdatingEmail(email)` - Verify before email update

**Authentication:**
- `Reauthenticate(credential)` - Re-authenticate user
- `ReauthenticateAndRetrieveData(credential)` - Re-authenticate with data
- `ReauthenticateWithProvider(provider)` - Re-authenticate with provider

**Account Linking:**
- `LinkWithCredential(credential)` - Link auth provider
- `LinkWithProvider(provider)` - Link federated provider
- `Unlink(provider)` - Unlink provider

**Account Management:**
- `Reload()` - Refresh user data
- `Delete()` - Delete user account

---

#### `firebase::auth::Credential`
Authentication credential for providers.

**Static Factory Methods:**
- Email provider credentials
- OAuth provider credentials
- Phone auth credentials
- Game Center credentials (iOS)
- Play Games credentials (Android)

---

#### Supporting Types

**`UserInfoInterface`**
- Base interface for user information
- Methods: `uid()`, `email()`, `display_name()`, `photo_url()`, `provider_id()`, `phone_number()`

**`AuthResult`**
- `user` - The signed-in user
- `additional_user_info` - Additional provider data

**`AdditionalUserInfo`**
- `provider_id` - Provider identifier
- `user_name` - Provider username
- `profile` - Provider profile data

**`UserMetadata`**
- `last_sign_in_timestamp` - Last sign-in time
- `creation_timestamp` - Account creation time

**`UserProfile`**
- `display_name` - Display name to update
- `photo_url` - Photo URL to update

**`FetchProvidersResult`**
- `providers` - List of available provider IDs

---

## Firestore Module APIs

### Core Classes

#### `firebase::firestore::Firestore`
Main Firestore database instance.

**Static Methods:**
- `GetInstance(App*, InitResult*)` - Get Firestore for app
- `GetInstance(App*, db_name, InitResult*)` - Get Firestore with custom DB name
- `GetInstance(InitResult*)` - Get default Firestore
- `set_log_level(level)` - Set logging level

**Instance Methods:**
- `app()` - Get associated Firebase App
- `Collection(path)` - Get collection reference
- `Document(path)` - Get document reference
- `CollectionGroup(collection_id)` - Query across collection groups
- `batch()` - Create write batch
- `RunTransaction(callback)` - Run transaction
- `RunTransaction(options, callback)` - Run transaction with options

**Settings:**
- `settings()` - Get current settings
- `set_settings(settings)` - Configure Firestore

**Network:**
- `DisableNetwork()` - Disable network access
- `EnableNetwork()` - Enable network access
- `Terminate()` - Terminate instance
- `WaitForPendingWrites()` - Wait for writes to complete
- `ClearPersistence()` - Clear offline cache

**Bundles:**
- `LoadBundle(bundle_data, progress_listener)` - Load data bundle
- `NamedQuery(query_name)` - Get named query from bundle

---

#### `firebase::firestore::DocumentReference`
Reference to a document location.

**Properties:**
- `id()` - Document ID
- `path()` - Full document path
- `firestore()` - Parent Firestore instance
- `Parent()` - Parent collection reference

**Collection Access:**
- `Collection(path)` - Get subcollection

**Read Operations:**
- `Get(source)` - Get document snapshot
- `AddSnapshotListener(listener)` - Listen to document changes
- `AddSnapshotListener(metadata_changes, listener)` - Listen with metadata options

**Write Operations:**
- `Set(data)` - Set document data (overwrite)
- `Set(data, options)` - Set with merge options
- `Update(data)` - Update specific fields
- `Update(field, value, ...)` - Update with field/value pairs
- `Delete()` - Delete document

---

#### `firebase::firestore::CollectionReference`
Reference to a collection (extends Query).

**Properties:**
- `id()` - Collection ID
- `path()` - Full collection path
- `Parent()` - Parent document reference

**Document Management:**
- `Document()` - Generate document with auto-ID
- `Document(path)` - Get document reference

**Write Operations:**
- `Add(data)` - Add new document with auto-ID

---

#### `firebase::firestore::Query`
Query for documents.

**Filtering:**
- `Where(field, op, value)` - Filter by field condition
- `Where(path, op, value)` - Filter by field path
- `Where(filter)` - Apply filter object
- `WhereEqualTo(field, value)` - Equality filter
- `WhereNotEqualTo(field, value)` - Inequality filter
- `WhereGreaterThan(field, value)` - Greater than filter
- `WhereGreaterThanOrEqualTo(field, value)` - Greater/equal filter
- `WhereLessThan(field, value)` - Less than filter
- `WhereLessThanOrEqualTo(field, value)` - Less/equal filter
- `WhereArrayContains(field, value)` - Array contains filter
- `WhereArrayContainsAny(field, values)` - Array contains any filter
- `WhereIn(field, values)` - Value in array filter
- `WhereNotIn(field, values)` - Value not in array filter

**Ordering:**
- `OrderBy(field)` - Order by field (ascending)
- `OrderBy(field, direction)` - Order by field with direction
- `OrderBy(field_path)` - Order by field path
- `OrderBy(field_path, direction)` - Order by path with direction

**Limiting:**
- `Limit(limit)` - Limit number of results
- `LimitToLast(limit)` - Limit to last N results

**Pagination:**
- `StartAt(snapshot)` - Start at document
- `StartAt(values...)` - Start at field values
- `StartAfter(snapshot)` - Start after document
- `StartAfter(values...)` - Start after field values
- `EndBefore(snapshot)` - End before document
- `EndBefore(values...)` - End before field values
- `EndAt(snapshot)` - End at document
- `EndAt(values...)` - End at field values

**Execution:**
- `Get(source)` - Execute query and get snapshot
- `AddSnapshotListener(listener)` - Listen to query results
- `AddSnapshotListener(metadata_changes, listener)` - Listen with options

**Aggregation:**
- `Count()` - Create count aggregation query

---

#### `firebase::firestore::WriteBatch`
Batch multiple write operations.

**Methods:**
- `Set(doc, data)` - Add set operation
- `Set(doc, data, options)` - Add set with options
- `Update(doc, data)` - Add update operation
- `Update(doc, field, value, ...)` - Add update with field/value pairs
- `Delete(doc)` - Add delete operation
- `Commit()` - Execute all operations atomically

---

#### `firebase::firestore::Transaction`
Atomic read-modify-write transaction.

**Read Operations:**
- `Get(doc)` - Read document in transaction

**Write Operations:**
- `Set(doc, data)` - Set document
- `Set(doc, data, options)` - Set with options
- `Update(doc, data)` - Update document
- `Update(doc, field, value, ...)` - Update with field/value pairs
- `Delete(doc)` - Delete document

---

#### `firebase::firestore::DocumentSnapshot`
Snapshot of document data.

**Properties:**
- `id()` - Document ID
- `reference()` - Document reference
- `metadata()` - Snapshot metadata
- `exists()` - Check if document exists

**Data Access:**
- `GetData()` - Get all document data as map
- `Get(field)` - Get specific field value
- `Get(field_path)` - Get field by path

---

#### `firebase::firestore::QuerySnapshot`
Result of a query execution.

**Properties:**
- `query()` - Original query
- `metadata()` - Snapshot metadata
- `documents()` - All document snapshots
- `document_changes()` - Changes since last snapshot
- `size()` - Number of documents

**Iteration:**
- Iterator support for documents

---

#### `firebase::firestore::FieldValue`
Represents field values and sentinel values.

**Factory Methods - Real Values:**
- `Null()` - Null value
- `Boolean(value)` - Boolean value
- `Integer(value)` - Integer value (int64_t)
- `Double(value)` - Double value
- `Timestamp(value)` - Timestamp value
- `String(value)` - String value
- `Blob(data, size)` - Binary blob
- `Reference(value)` - Document reference
- `GeoPoint(value)` - Geographic point
- `Array(values)` - Array of values
- `Map(value)` - Map of key-value pairs

**Factory Methods - Sentinel Values:**
- `Delete()` - Mark field for deletion
- `ServerTimestamp()` - Use server timestamp
- `ArrayUnion(elements)` - Union elements into array
- `ArrayRemove(elements)` - Remove elements from array
- `Increment(value)` - Increment numeric field

**Type Checking:**
- `type()` - Get value type
- `is_null()`, `is_boolean()`, `is_integer()`, `is_double()`, etc.

**Value Accessors:**
- `boolean_value()`, `integer_value()`, `double_value()`, `string_value()`, etc.

---

#### Supporting Types

**`FieldPath`**
- Represents path to a field
- `DocumentId()` - Special document ID path
- Constructor with field names

**`GeoPoint`**
- `latitude()` - Latitude value
- `longitude()` - Longitude value

**`Timestamp`**
- `seconds()` - Unix seconds
- `nanoseconds()` - Nanoseconds
- `Now()` - Current timestamp

**`Settings`**
- `host()` - Firestore host
- `set_host(host)` - Set host
- `is_ssl_enabled()` - SSL status
- `set_ssl_enabled(enabled)` - Configure SSL
- `is_persistence_enabled()` - Persistence status
- `set_persistence_enabled(enabled)` - Configure persistence
- `cache_size_bytes()` - Cache size
- `set_cache_size_bytes(size)` - Set cache size

**`SetOptions`**
- `Merge()` - Merge with existing data
- `MergeFields(fields)` - Merge specific fields
- `MergeFieldPaths(paths)` - Merge specific paths

**`Source`**
- `kDefault` - Default source
- `kServer` - Force server read
- `kCache` - Force cache read

**`MetadataChanges`**
- `kExclude` - Exclude metadata changes
- `kInclude` - Include metadata changes

**`SnapshotMetadata`**
- `has_pending_writes()` - Check for pending writes
- `is_from_cache()` - Check if from cache

**`Filter`**
- Filter object for complex queries
- `And(filters...)` - Logical AND
- `Or(filters...)` - Logical OR
- Field-based filters

**`DocumentChange`**
- `type()` - Change type (added/modified/removed)
- `document()` - Changed document
- `old_index()`, `new_index()` - Index changes

**`ListenerRegistration`**
- `Remove()` - Unregister listener

**`TransactionOptions`**
- `max_attempts()` - Max retry attempts
- `set_max_attempts(n)` - Set max attempts

**`AggregateQuery`**
- `Get(source)` - Execute aggregation

**`AggregateQuerySnapshot`**
- `count()` - Get count result
- `query()` - Original query

---

## Implementation Scope

**All APIs listed in this document will be implemented.**

The implementation will be driven by a dependency graph analysis using CodeQL to determine the correct implementation order, starting from leaf dependencies.
