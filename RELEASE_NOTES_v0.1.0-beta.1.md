# Release Notes: v0.1.0-beta.1

**Release Date**: December 17, 2024  
**Status**: Beta - Ready for Development & Testing

## 🎉 What's New

### Query Listeners (Phase 6) ✅
Real-time updates for documents and queries with automatic cleanup using Rust's RAII pattern.

```rust
use futures::StreamExt;

// Listen to document changes
let mut stream = doc_ref.listen(Some(MetadataChanges::Include));
while let Some(result) = stream.next().await {
    match result {
        Ok(snapshot) => println!("Document updated: {:?}", snapshot.data()),
        Err(e) => eprintln!("Error: {}", e),
    }
}
// Stream automatically cleans up when dropped

// Listen to query changes
let query = firestore.collection("users").where_equal_to("active", true.into());
let mut stream = query.listen(None);
while let Some(result) = stream.next().await {
    match result {
        Ok(snapshot) => println!("Query results: {} documents", snapshot.documents().len()),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

**Features:**
- DocumentSnapshot and QuerySnapshot streaming
- MetadataChanges support for metadata-only updates
- Automatic listener cleanup on Drop (no manual removal needed)
- Zero-copy forwarding from gRPC to Rust async streams

**Tests:** 4 integration tests passing

### Aggregation Queries (Phase 8) ✅
Efficient server-side aggregations without fetching all documents.

```rust
// Count documents
let count_result = firestore
    .collection("products")
    .where_greater_than("price", 100.0.into())
    .count()
    .get()
    .await?;
println!("Products over $100: {}", count_result.count().unwrap_or(0));

// Multiple aggregations
let result = firestore
    .collection("orders")
    .aggregate(vec![
        AggregateField::count(),
        AggregateField::sum("amount").with_alias("total_revenue"),
        AggregateField::average("amount").with_alias("avg_order"),
    ])
    .get()
    .await?;

println!("Total orders: {}", result.count().unwrap_or(0));
println!("Revenue: ${}", result.get_double("total_revenue").unwrap_or(0.0));
println!("Average: ${}", result.get_double("avg_order").unwrap_or(0.0));
```

**Features:**
- COUNT, SUM, AVERAGE operations
- Custom aliases for results
- Type-safe result extraction (get_long, get_double)
- Works with any query (filters, ordering, etc.)

**Tests:** 4 integration tests passing

### Settings Configuration (Phase 7) ✅
Configure Firestore connection settings at initialization.

```rust
use firebase_rust_sdk::firestore::{Firestore, Settings};

let settings = Settings {
    host: "firestore.googleapis.com".to_string(),
    ssl_enabled: true,
    persistence_enabled: false,
    cache_size_bytes: 100 * 1024 * 1024, // 100 MB
};

let firestore = Firestore::with_settings(
    "my-project",
    "(default)",
    settings,
    Some(id_token),
).await?;
```

**Features:**
- Custom host configuration
- SSL enable/disable
- Cache size configuration
- Persistence flag (for future implementation)

## ⚡ Performance Improvements

### Proto Type Integration
Refactored to use Protocol Buffer types directly, eliminating conversion overhead:

- **Before**: Custom enums → proto enums (match statements for every operation)
- **After**: Proto enums directly (`*operator as i32`)
- **Result**: Faster query execution, smaller binary size

### Memory Optimization
- Arc<AuthData> for cheaper clones across requests
- Channel reuse for gRPC connections
- Reduced allocations in hot paths

## 🔧 Breaking Changes

### Proto Types (Easy Migration)
```rust
// Before (alpha.1)
use firebase_rust_sdk::firestore::{Direction, FilterOperator};
query.order_by("name", Direction::Ascending);

// After (beta.1) - same API, different import
use firebase_rust_sdk::firestore::Direction;
query.order_by("name", Direction::Ascending);
```

**Migration:** Direction is now re-exported from proto in the query module. Your code will continue to work, just update imports if you used fully-qualified paths.

## 🧹 API Cleanup

### Removed
- Unused internal methods (`_get_auth_token`, `_operations`)
- Custom FilterOperator enum (use proto types directly)
- OrderDirection enum (replaced by Direction)

### Improved
- Internal query methods now properly hidden with `#[doc(hidden)]`
- Transaction field naming more accurate
- Better documentation with C++ SDK references

## 📊 Quality Metrics

- ✅ **35 integration tests** - all passing
- ✅ **Zero compiler warnings** - clean build
- ✅ **Zero clippy warnings** (with intentional allows)
- ✅ **Documentation coverage** - all public APIs documented
- ✅ **C++ SDK parity** - Phases 1-8 complete

## 📦 What's Included

### Firebase Authentication
- Email/password, anonymous, OAuth providers
- Custom token authentication
- Automatic token refresh
- User management
- Auth state listeners

### Cloud Firestore
- Document CRUD operations
- Advanced queries (filters, ordering, pagination)
- Real-time listeners (documents & queries) ⭐ NEW
- Aggregation queries (COUNT, SUM, AVERAGE) ⭐ NEW
- Settings configuration ⭐ NEW
- WriteBatch for atomic operations
- Transactions with automatic retry
- Nested collections
- GeoPoint, Timestamp support

### Infrastructure
- Full async/await with Tokio
- gRPC transport with TLS
- Comprehensive error handling
- Integration tests

## 🚀 Publishing Checklist

- [x] All tests passing (35/35)
- [x] Zero warnings on cargo check
- [x] Documentation updated
- [x] CHANGELOG.md updated
- [x] README.md updated
- [x] Version bumped to 0.1.0-beta.1
- [x] Git tagged: v0.1.0-beta.1
- [x] Merged to main branch
- [ ] Push to remote: `git push origin main --tags`
- [ ] Publish to crates.io: `cargo publish`

## 🎯 Next Steps

### For Users
1. Update dependency: `firebase-rust-sdk = "0.1.0-beta.1"`
2. Test new listener and aggregation features
3. Report issues on GitHub
4. Provide feedback on API ergonomics

### For Development
1. Implement Phase 9: Load Bundles (optional)
2. Add offline persistence (Phase 10)
3. Optimize performance further
4. Expand integration test coverage
5. Move toward 1.0.0 stable release

## 📝 Known Limitations

- Offline persistence has API structure but not implemented
- DocumentChange tracking not exposed (internal only)
- Some advanced query features pending (OR queries, IN with arrays)
- No connection pooling yet

## 🐛 Bug Reports

Please report issues at: https://github.com/cwahn/firebase-rust-sdk/issues

Include:
- Rust version
- Firebase project configuration
- Minimal reproduction example
- Error messages or unexpected behavior

## 📄 License

MIT License - see LICENSE file for details

## 🙏 Acknowledgments

This project is an unofficial port of the Firebase C++ SDK. Thanks to:
- Firebase team for the excellent C++ SDK
- Rust community for async/gRPC ecosystem
- Contributors and testers

---

**Ready to publish?**  
```bash
# Push changes
git push origin main --tags

# Publish to crates.io
cargo publish
```
