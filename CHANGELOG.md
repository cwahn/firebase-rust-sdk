# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0-beta] - 2024-12-17

### Added
- **Query Listeners**: Real-time query snapshot listeners with automatic cleanup
- **Aggregation Queries**: COUNT, SUM, AVERAGE operations with custom aliases
- **Settings Configuration**: Firestore settings with host, SSL, and cache configuration
- Integration tests for all new features (35 tests passing)

### Changed
- **Proto Type Integration**: Refactored to use proto types directly for better performance
  - Removed custom Direction and FilterOperator enums
  - Direction re-exported from proto in query module (matches C++ SDK Query::Direction)
  - Internal type aliases for cleaner code (CompositeFilterOp, FieldFilterOp)
- **API Cleanup**: Removed unused methods and improved code organization
- **Performance**: Eliminated enum conversions, direct proto usage reduces overhead

### Fixed
- All compiler warnings resolved
- Transaction field visibility and naming
- Query listener cleanup on drop
- Proto type compatibility across all modules

## [0.1.0-alpha.1] - 2024-12-16

### Added

#### Firebase Authentication
- Email/password authentication (sign in, create user)
- Anonymous authentication
- OAuth providers (Google, Facebook, GitHub, generic OAuth)
- Custom token authentication (server-side JWT)
- Password reset email
- Automatic token refresh with expiration tracking
- User management (update_password, update_email, delete, update_profile)
- Auth state change listeners (async streams)
- Sign out functionality
- Current user tracking

#### Cloud Firestore
- Document CRUD operations (Get, Set, Update, Delete) via gRPC
- Query operations (filters, ordering, limits)
- Query pagination (start_at, start_after, end_at, end_before)
- CollectionReference::add() with auto-generated IDs
- WriteBatch for atomic multi-document operations
- Transactions with automatic retry logic
- Real-time snapshot listeners using gRPC streaming (idiomatic Rust Streams)
- DocumentReference, DocumentSnapshot, QuerySnapshot types
- GeoPoint, Timestamp field types
- Nested collections support
- Path-based document access
- Compound filters (And/Or with nesting)
- Offline persistence API structure (implementation pending)

#### Infrastructure
- Full async/await support with Tokio
- gRPC transport with TLS for Firestore
- REST API for Authentication
- Comprehensive error handling
- 24 integration tests
- docs.rs documentation

### Notes
- This is an alpha release. APIs may change before 1.0.0
- Firestore offline persistence has API structure but implementation is pending
- Requires Firebase project with Authentication and Firestore enabled
- Cross-platform support (native and WASM target structure in place)

[0.1.0-alpha.1]: https://github.com/cwahn/firebase-rust-sdk/releases/tag/v0.1.0-alpha.1
