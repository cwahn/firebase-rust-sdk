# What You Need to Prepare for Integration Tests

## ✅ Checklist

### 1. Firebase Project Setup (5 minutes)

**Go to:** https://console.firebase.google.com/

#### Create/Select Project
- [ ] Create a new Firebase project (or use existing)
- [ ] Note down your **Project ID** (e.g., `my-project-123abc`)

#### Enable Authentication
- [ ] Go to **Authentication** → **Sign-in method**
- [ ] Enable **Email/Password** provider
- [ ] Enable **Anonymous** provider
- [ ] (Optional) Enable **Google** provider for OAuth tests

#### Create Test User
- [ ] Go to **Authentication** → **Users**
- [ ] Click **Add user**
- [ ] Email: `test@example.com` (or your choice)
- [ ] Password: `TestPassword123!` (or your choice)
- [ ] Save the credentials

#### Get API Key
- [ ] Go to **Project Settings** (gear icon)
- [ ] Scroll to **Web API Key**
- [ ] Copy the key (looks like `AIzaSyC...`)

### 2. Firestore Setup (2 minutes)

**In Firebase Console:**

#### Create Database
- [ ] Go to **Firestore Database**
- [ ] Click **Create Database**
- [ ] Choose **Test mode** (for development)
- [ ] Select nearest region

#### Security Rules (Recommended)
- [ ] Go to **Firestore Database** → **Rules**
- [ ] Use this for testing:

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

### 3. Local Configuration (1 minute)

**In your terminal:**

```bash
# Switch to integration-tests branch
git checkout integration-tests

# Copy environment template
cp .env.example .env

# Edit with your credentials
nano .env
```

**Fill in your `.env` file:**
```bash
FIREBASE_API_KEY=AIzaSyC-your-actual-key-here
FIREBASE_PROJECT_ID=your-project-id
TEST_USER_EMAIL=test@example.com
TEST_USER_PASSWORD=TestPassword123!
```

### 4. Verify Setup (1 minute)

```bash
# Run integration tests
cargo test --features integration-tests -- --test-threads=1
```

**Expected output:**
```
running 20 tests
test test_sign_in_with_email_password ... ok
test test_anonymous_auth ... ok
test test_create_and_delete_user ... ok
...
✅ All 20 tests passed!
```

## 📋 Required Information Summary

You need to provide these 4 values:

| Variable | Where to Find | Example |
|----------|---------------|---------|
| `FIREBASE_API_KEY` | Firebase Console → Project Settings → Web API Key | `AIzaSyC...` |
| `FIREBASE_PROJECT_ID` | Firebase Console → Project Settings → Project ID | `my-app-123abc` |
| `TEST_USER_EMAIL` | Create in Authentication → Users | `test@example.com` |
| `TEST_USER_PASSWORD` | Set when creating test user | `TestPassword123!` |

## 🚀 Quick Start Commands

```bash
# 1. Switch to integration tests branch
git checkout integration-tests

# 2. Setup environment
cp .env.example .env
# (edit .env with your credentials)

# 3. Run all tests
cargo test --features integration-tests -- --test-threads=1

# 4. Run only auth tests
cargo test --features integration-tests auth_integration -- --test-threads=1

# 5. Run only firestore tests
cargo test --features integration-tests firestore_integration -- --test-threads=1

# 6. Run with full output
cargo test --features integration-tests -- --test-threads=1 --nocapture
```

## 💡 Tips

### Use a Test Project
- Don't use your production Firebase project
- Create a separate project for testing
- This keeps your production data safe

### Test Mode for Firestore
- Use "Test mode" when creating Firestore database
- This allows all authenticated reads/writes
- Perfect for development and testing

### Save Your Credentials
- Keep your `.env` file secure
- Don't commit it to git (already in `.gitignore`)
- You can create multiple `.env` files for different projects

### Monitor Usage
- Firebase Console → Usage
- Free tier includes:
  - 50K reads/day
  - 20K writes/day
  - 20K deletes/day
- Tests use ~200 operations per full run

## 🔍 Troubleshooting

### Can't find API key?
**Solution:** Firebase Console → Project Settings → Scroll down to "Web API Key"

### Tests fail with "Invalid API key"?
**Solution:** Double-check the key in `.env`, make sure there are no extra spaces

### Tests fail with "Permission denied"?
**Solution:** 
1. Make sure you're signed in (auth tests should pass first)
2. Check Firestore security rules allow authenticated access

### Tests fail with "User not found"?
**Solution:** Create the test user in Firebase Console → Authentication → Users

### Tests timeout?
**Solution:** 
1. Check your internet connection
2. Make sure `--test-threads=1` is used
3. Some tests (listeners) take a few seconds

## 📚 Documentation

- **Detailed Setup:** See `INTEGRATION_TESTS.md`
- **Quick Reference:** See `INTEGRATION_TESTS_BRANCH.md`
- **Example .env:** See `.env.example`

## 🎯 What Gets Tested

### Authentication (9 tests)
✅ Sign in with email/password  
✅ Anonymous authentication  
✅ Create and delete users  
✅ Token refresh  
✅ Update user profile  
✅ Password reset  
✅ User reload  
✅ Email verification  
✅ Password update  

### Firestore (11 tests)
✅ Create/read/update/delete documents  
✅ Query with filters  
✅ Query pagination  
✅ Batch writes  
✅ Transactions  
✅ Auto-generated IDs  
✅ Nested collections  
✅ Real-time listeners  
✅ Compound filters  

## ✨ After Setup

Once configured, you can:
- Run tests anytime to verify SDK functionality
- Add new integration tests for features
- Use in CI/CD pipeline
- Validate changes before deploying

## 🔒 Security Reminder

⚠️ **Never commit `.env` file to git!**

It's already in `.gitignore`, but be careful when sharing your code:
- Don't screenshot your `.env` file
- Don't share terminal output showing credentials
- Rotate API keys if accidentally exposed

---

**Time to setup:** ~10 minutes  
**Tests duration:** ~30 seconds  
**Cost:** Free (within Firebase free tier)
