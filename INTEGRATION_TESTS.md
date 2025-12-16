# Integration Tests Setup Guide

## Prerequisites

### 1. Firebase Project Setup

1. Go to [Firebase Console](https://console.firebase.google.com/)
2. Create a new project or use an existing one
3. Note down your **Project ID**

### 2. Enable Authentication

1. In Firebase Console, go to **Authentication** > **Sign-in method**
2. Enable the following providers:
   - ✅ **Email/Password** (required)
   - ✅ **Anonymous** (required)
   - ✅ **Google** (optional, for OAuth tests)

### 3. Enable Firestore

1. In Firebase Console, go to **Firestore Database**
2. Click **Create Database**
3. Choose **Test mode** (or **Production mode** and set up rules)
4. Select a region close to you

### 4. Get API Key

1. In Firebase Console, go to **Project Settings** (gear icon)
2. Scroll down to **Web API Key**
3. Copy this key

### 5. Create Test User

1. Go to **Authentication** > **Users**
2. Click **Add user**
3. Create a test user:
   - Email: `test@example.com` (or your choice)
   - Password: `TestPassword123!` (or your choice)
4. Note down these credentials

### 6. Firestore Security Rules (Recommended)

For testing, use permissive rules (⚠️ NOT for production):

```javascript
rules_version = '2';
service cloud.firestore {
  match /databases/{database}/documents {
    match /{document=**} {
      allow read, write: if request.auth != null;
    }
  }
}
```

### 7. Configure Environment Variables

1. Copy `.env.example` to `.env`:
   ```bash
   cp .env.example .env
   ```

2. Fill in your Firebase credentials in `.env`:
   ```bash
   FIREBASE_API_KEY=AIzaSyC...
   FIREBASE_PROJECT_ID=my-project-id
   TEST_USER_EMAIL=test@example.com
   TEST_USER_PASSWORD=TestPassword123!
   ```

3. **Important**: Add `.env` to `.gitignore` (already done)

## Running Integration Tests

### Run all integration tests:
```bash
cargo test --features integration-tests -- --test-threads=1
```

**Note**: `--test-threads=1` ensures tests run sequentially to avoid conflicts.

### Run specific test module:
```bash
# Auth tests only
cargo test --features integration-tests auth_integration -- --test-threads=1

# Firestore tests only
cargo test --features integration-tests firestore_integration -- --test-threads=1
```

### With output:
```bash
cargo test --features integration-tests -- --test-threads=1 --nocapture
```

## What Gets Tested

### Authentication Tests
- ✅ Email/password sign in
- ✅ Create new user account
- ✅ Anonymous authentication
- ✅ Password reset email
- ✅ Token refresh
- ✅ User profile updates
- ✅ User deletion
- ✅ Sign out

### Firestore Tests
- ✅ Create documents
- ✅ Read documents
- ✅ Update documents
- ✅ Delete documents
- ✅ Query with filters
- ✅ Query with pagination
- ✅ Collections and subcollections
- ✅ Batch writes
- ✅ Transactions
- ✅ Real-time listeners

## Cleanup

All tests automatically clean up after themselves:
- Documents created during tests are deleted
- Test users created during tests are deleted (except the main test user)
- Collections use timestamped names to avoid conflicts

## Troubleshooting

### "Invalid API key" error
- Double-check your API key in `.env`
- Make sure it's the Web API Key from Firebase Console

### "User not found" error
- Create the test user in Firebase Console
- Verify credentials in `.env`

### "Permission denied" error
- Check Firestore security rules
- Make sure authentication is working
- User must be signed in for Firestore operations

### "Network error" or timeout
- Check your internet connection
- Verify Firebase project is active
- Check Firebase quota limits

### Tests hang or timeout
- Use `--test-threads=1` to run tests sequentially
- Some operations (like listeners) take a few seconds

## Security Notes

⚠️ **Important Security Practices:**

1. **Never commit `.env` file** - It's in `.gitignore`
2. **Use a separate test project** - Don't use production Firebase project
3. **Limit test user permissions** - Use Firestore security rules
4. **Rotate API keys regularly** - Especially if exposed
5. **Use test mode** - For development, use Firestore test mode
6. **Monitor usage** - Check Firebase Console for unexpected activity

## Cost Considerations

Firebase free tier (Spark plan) includes:
- ✅ 50,000 document reads/day
- ✅ 20,000 document writes/day
- ✅ 20,000 document deletes/day
- ✅ 10 GB storage

Integration tests typically use:
- ~50-100 reads per full test run
- ~30-50 writes per full test run
- ~30-50 deletes per full test run

**Cost**: Free tier is sufficient for development and CI/CD.

## CI/CD Integration

### GitHub Actions Example:

```yaml
name: Integration Tests

on:
  push:
    branches: [main, integration-tests]
  pull_request:
    branches: [main]

jobs:
  integration:
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

**Setup**: Add secrets in GitHub repo settings > Secrets and variables > Actions

## Support

If you encounter issues:
1. Check Firebase Console for error messages
2. Review Firebase quota and billing
3. Verify all prerequisites are met
4. Check test output with `--nocapture` flag
