# Publishing Guide: v0.1.0-beta.1

## Pre-Publishing Verification ✅

All checks completed successfully:

- [x] **Code Quality**
  - Zero compiler warnings
  - Zero clippy warnings (with intentional allows)
  - All tests passing (35/35 integration tests)
  
- [x] **Documentation**
  - README.md updated with beta.1 version
  - CHANGELOG.md has beta.1 entry
  - FIRESTORE_IMPLEMENTATION_PLAN.md updated (Phases 6-8 complete)
  - Release notes created
  - All public APIs documented
  
- [x] **Version Control**
  - Version bumped to 0.1.0-beta.1 in Cargo.toml
  - All changes committed
  - Feature branch merged to main
  - Git tag created: v0.1.0-beta.1
  
- [x] **Package Validation**
  - Cargo.toml metadata complete
  - LICENSE file present (MIT)
  - README.md present
  - exclude list configured (no C++ SDK in package)

## Publishing Steps

### 1. Push to GitHub

```bash
# Push main branch with all commits
git push origin main

# Push tags
git push origin --tags

# Verify on GitHub
# - Check that v0.1.0-beta.1 tag is visible
# - Create a GitHub Release from the tag (optional but recommended)
```

### 2. Publish to crates.io

```bash
# Dry run first (validate package)
cargo publish --dry-run

# If dry run succeeds, publish for real
cargo publish

# Note: You need to be logged in to crates.io
# If not logged in: cargo login <your-api-token>
```

### 3. Verify Publication

After publishing:

1. Visit https://crates.io/crates/firebase-rust-sdk
2. Verify version 0.1.0-beta.1 is listed
3. Check documentation at https://docs.rs/firebase-rust-sdk/0.1.0-beta.1

### 4. Announce Release (Optional)

Consider announcing on:
- GitHub Releases page
- Reddit r/rust
- Rust Discord
- Twitter/X
- This Week in Rust

## Post-Publishing

### Create GitHub Release

1. Go to https://github.com/cwahn/firebase-rust-sdk/releases/new
2. Choose tag: v0.1.0-beta.1
3. Title: "Release v0.1.0-beta.1: Complete Firestore API"
4. Description: Use content from RELEASE_NOTES_v0.1.0-beta.1.md
5. Mark as "Pre-release" (since it's beta)
6. Publish release

### Monitor

- Watch for issues on GitHub
- Check docs.rs build status
- Monitor download stats on crates.io

## Troubleshooting

### If cargo publish fails:

**Missing API token:**
```bash
# Get token from https://crates.io/settings/tokens
cargo login <your-token>
```

**Package too large:**
```bash
# Check package size
cargo package --list | wc -l

# Review exclude list in Cargo.toml
# Currently excludes: firebase-cpp-sdk/, docs/, analysis files
```

**Dependency issues:**
```bash
# Update dependencies if needed
cargo update
cargo check
cargo test
```

### If docs.rs build fails:

- Check build.rs for platform-specific issues
- Verify all dependencies support docs.rs
- Check for missing features in Cargo.toml

## Rollback Procedure

If critical issues are discovered:

1. **Yank the version** (doesn't delete, prevents new users from using it):
   ```bash
   cargo yank --vers 0.1.0-beta.1
   ```

2. **Fix issues** in a new branch

3. **Publish patch version**:
   - Bump to 0.1.0-beta.2
   - Repeat publishing process

Note: Yanked versions can be un-yanked if issues are resolved:
```bash
cargo yank --vers 0.1.0-beta.1 --undo
```

## Success Criteria

After publishing, consider it successful when:

- ✅ Package appears on crates.io
- ✅ Documentation builds on docs.rs
- ✅ `cargo install firebase-rust-sdk --version 0.1.0-beta.1` works
- ✅ Example projects can use the published crate
- ✅ No critical issues reported in first 24 hours

## Next Development Cycle

After successful publish:

1. Create milestone for v0.1.0 (stable)
2. Gather feedback from beta users
3. Fix any reported issues
4. Consider implementing:
   - Phase 9: Load Bundles (optional)
   - Phase 10: Offline Persistence
   - Additional integration tests
   - Performance optimizations

## Contact

Maintainer: Chan Woo Ahn  
Repository: https://github.com/cwahn/firebase-rust-sdk  
Issues: https://github.com/cwahn/firebase-rust-sdk/issues

---

**Ready to publish?**

```bash
# Final checks
cargo test --test firestore_integration -- --test-threads=35
cargo check
cargo clippy

# Push to GitHub
git push origin main --tags

# Publish to crates.io
cargo publish
```

Good luck! 🚀
