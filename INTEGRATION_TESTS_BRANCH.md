# Integration Tests Branch

This branch contains real Firebase/Firestore integration tests. These tests interact with actual Firebase services to verify the SDK works correctly in production scenarios.

## Quick Start

### 1. Setup Firebase Project

Follow the detailed instructions in [INTEGRATION_TESTS.md](./INTEGRATION_TESTS.md)

**Quick checklist:**
- ✅ Create Firebase project
- ✅ Enable Email/Password authentication
- ✅ Enable Anonymous authentication
- ✅ Create Firestore database
- ✅ Create test user account
- ✅ Get API key and Project ID

### 2. Configure Environment

```bash
# Copy the example file
cp .env.example .env

# Edit .env with your Firebase credentials
nano .env
```

Required variables:
```bash
FIREBASE_API_KEY=AIzaSyC...
FIREBASE_PROJECT_ID=my-project-id
TEST_USER_EMAIL=test@example.com
TEST_USER_PASSWORD=TestPassword123!
```

### 3. Run Tests

```bash
# Run all integration tests
cargo test --features integration-tests -- --test-threads=1

# Run only auth tests
cargo test --features integration-tests auth_integration -- --test-threads=1

# Run only firestore tests
cargo test --features integration-tests firestore_integration -- --test-threads=1

# With output
cargo test --features integration-tests -- --test-threads=1 --nocapture
```

## Test Coverage

### Authentication Tests (9 tests)
- ✅ `test_sign_in_with_email_password` - Email/password authentication
- ✅ `test_anonymous_auth` - Anonymous user creation
- ✅ `test_create_and_delete_user` - User lifecycle
- ✅ `test_token_refresh` - ID token refresh
- ✅ `test_update_profile` - Display name update
- ✅ `test_password_reset` - Password reset email
- ✅ `test_user_reload` - Refresh user data
- ✅ `test_send_email_verification` - Email verification
- ✅ `test_update_password` - Password change

### Firestore Tests (11 tests)
- ✅ `test_create_read_document` - Basic CRUD
- ✅ `test_update_document` - Document updates
- ✅ `test_delete_document` - Document deletion
- ✅ `test_query_filters` - Filtered queries
- ✅ `test_query_pagination` - Paginated queries
- ✅ `test_batch_writes` - Atomic batch operations
- ✅ `test_transactions` - Atomic transactions
- ✅ `test_add_document` - Auto-generated IDs
- ✅ `test_nested_collections` - Subcollections
- ✅ `test_snapshot_listener` - Real-time updates
- ✅ `test_compound_filters` - And/Or filters

**Total: 20 integration tests**

## Features

### Automatic Cleanup
All tests clean up after themselves:
- Temporary users are deleted
- Test documents are removed
- Collections use timestamped names

### Error Handling
Tests use descriptive error messages:
```rust
.expect("Failed to sign in") // Clear error context
```

### Real Firebase APIs
Tests use actual Firebase REST APIs:
- `identitytoolkit.googleapis.com` for auth
- `firestore.googleapis.com` for database

### Sequential Execution
Tests run with `--test-threads=1` to avoid:
- Race conditions
- Resource conflicts
- Rate limiting

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run integration tests
        env:
          FIREBASE_API_KEY: ${{ secrets.FIREBASE_API_KEY }}
          FIREBASE_PROJECT_ID: ${{ secrets.FIREBASE_PROJECT_ID }}
          TEST_USER_EMAIL: ${{ secrets.TEST_USER_EMAIL }}
          TEST_USER_PASSWORD: ${{ secrets.TEST_USER_PASSWORD }}
        run: |
          cargo test --features integration-tests -- --test-threads=1
```

**Setup:** Add secrets in GitHub Settings > Secrets and variables > Actions

## Security

⚠️ **Important:**
- Never commit `.env` file (it's in `.gitignore`)
- Use a dedicated test Firebase project
- Don't use production credentials
- Rotate API keys if exposed
- Monitor Firebase Console for unusual activity

## Troubleshooting

### Tests fail with "Invalid API key"
```bash
# Check your .env file
cat .env

# Verify API key in Firebase Console
# Project Settings > Web API Key
```

### Tests fail with "Permission denied"
```bash
# Check Firestore security rules
# Make sure authenticated users can read/write
```

### Tests timeout or hang
```bash
# Use sequential execution
cargo test --features integration-tests -- --test-threads=1

# Check network connection
# Check Firebase project status
```

### "User not found" error
```bash
# Create test user in Firebase Console
# Authentication > Users > Add user
```

## Cost

Firebase free tier (Spark plan) limits:
- 50,000 reads/day ✅
- 20,000 writes/day ✅
- 20,000 deletes/day ✅
- 10 GB storage ✅

Running all integration tests:
- ~100 reads per run
- ~50 writes per run
- ~50 deletes per run

**Free tier is sufficient for development!**

## Development Workflow

```bash
# 1. Create feature branch
git checkout -b my-feature

# 2. Make changes to SDK
# ... edit src/ files ...

# 3. Run unit tests
cargo test

# 4. Run integration tests
cargo test --features integration-tests -- --test-threads=1

# 5. Commit and push
git add -A
git commit -m "feat: Add new feature"
git push
```

## Adding New Tests

```rust
#[tokio::test]
async fn test_my_new_feature() {
    let (auth, firestore, _) = get_authenticated_firestore().await;
    let collection = test_collection("my_feature");
    
    // Test logic here
    
    // Clean up
    cleanup_collection(&firestore, &collection).await;
    auth.sign_out().await.expect("Failed to sign out");
    
    println!("✅ My new feature works!");
}
```

## Support

See [INTEGRATION_TESTS.md](./INTEGRATION_TESTS.md) for detailed setup instructions.

For issues:
1. Check Firebase Console for errors
2. Verify credentials in `.env`
3. Review test output with `--nocapture`
4. Check Firebase quota and billing

---

**Branch:** `integration-tests`  
**Status:** Ready for testing  
**Tests:** 20 integration tests (9 auth + 11 firestore)
