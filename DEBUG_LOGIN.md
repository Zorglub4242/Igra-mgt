# Debugging Login Issues

## Current Issue
Getting "Unexpected token '<', "<!doctype "..." error when trying to login with admin/admin

This means the login endpoint is returning HTML instead of JSON.

## Quick Fix Steps

### 1. Rebuild Everything

```bash
cd /home/kaspa/igra2/igra-orchestra-public/tools/igra-cli

# Rebuild frontend
cd igra-web-ui
npm run build
cd ..

# Rebuild backend (release mode)
cargo build --release --features server
```

### 2. Stop Any Running Servers

```bash
# Find and kill any running igra-cli server processes
pkill -f "igra-cli.*server"

# Or find the process ID
ps aux | grep "igra-cli.*server"
# Then kill it
kill <PID>
```

### 3. Start Fresh Server

```bash
# Start the server
./target/release/igra-cli server --host 0.0.0.0 --port 9000

# You should see:
# ⚠️  No users found. Creating default admin user...
#    Default password: admin
#    ⚠️  CHANGE THIS PASSWORD IMMEDIATELY!
# ✓ Default admin user created
# 🚀 Server running on http://0.0.0.0:9000
```

### 4. Test the Login Endpoint Directly

```bash
# Test with curl (in another terminal)
curl -X POST http://localhost:9000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin"}' \
  -v

# Expected response (should be JSON, not HTML):
# {"success":true,"data":{"username":"admin","roles":["Admin"]}}
```

### 5. Test in Browser

1. Open browser to http://localhost:9000
2. Open browser DevTools (F12)
3. Go to Network tab
4. Try logging in with admin/admin
5. Check the /api/auth/login request:
   - Should be POST request
   - Content-Type should be application/json
   - Response should be JSON, not HTML

## Common Issues

### Issue: Still Getting HTML Response

**Possible causes:**
1. Old binary still running - make sure you killed ALL igra-cli processes
2. Browser cache - hard refresh (Ctrl+Shift+R or Cmd+Shift+R)
3. Old frontend bundle - verify igra-web-ui/dist was updated

**Solution:**
```bash
# Nuclear option - rebuild everything from scratch
cd /home/kaspa/igra2/igra-orchestra-public/tools/igra-cli
cargo clean
cd igra-web-ui && rm -rf dist node_modules && npm install && npm run build && cd ..
cargo build --release --features server
```

### Issue: "Connection Refused"

Server isn't running. Start it:
```bash
./target/release/igra-cli server --host 0.0.0.0 --port 9000
```

### Issue: "404 Not Found" on API Routes

Routes aren't registered properly. Check:
1. Make sure you built with `--features server`
2. Check server logs for any errors on startup

### Issue: CORS Errors

If testing from different origin, add CORS flag:
```bash
./target/release/igra-cli server --host 0.0.0.0 --port 9000 --cors
```

## Debug Logging

Enable debug logs to see what's happening:

```bash
RUST_LOG=debug ./target/release/igra-cli server --host 0.0.0.0 --port 9000
```

This will show:
- All incoming requests
- Route matching
- Authentication attempts
- Any errors

## Verify Build Artifacts

```bash
# Check frontend was built
ls -lh igra-web-ui/dist/
# Should see: index.html, assets/

# Check binary exists and is recent
ls -lh target/release/igra-cli
# Should show recent timestamp

# Verify embedded assets
strings target/release/igra-cli | grep "<!DOCTYPE html" | head -1
# Should show HTML content (means frontend is embedded)
```

## Manual Test Sequence

Once server is running:

```bash
# 1. Health check (should return JSON)
curl http://localhost:9000/api/health

# 2. Version check (should return JSON)
curl http://localhost:9000/api/version

# 3. Session check (should return JSON - not authenticated)
curl http://localhost:9000/api/auth/session

# 4. Login (should return JSON with session cookie)
curl -c cookies.txt -X POST http://localhost:9000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin"}'

# 5. Session check with cookie (should return JSON - authenticated)
curl -b cookies.txt http://localhost:9000/api/auth/session

# 6. Protected endpoint (should work with cookie)
curl -b cookies.txt http://localhost:9000/api/services
```

## Expected vs Actual

### Expected Behavior:
- POST to `/api/auth/login` returns JSON: `{"success":true,"data":{...}}`
- Browser successfully logs in
- Redirects to dashboard

### Actual Behavior (Current Issue):
- POST to `/api/auth/login` returns HTML (index.html)
- Browser shows "Unexpected token '<'" error
- Login fails

### Root Cause:
The API route `/api/auth/login` is not being matched, so the request falls through to the static file handler, which returns index.html for all unmatched routes (for React Router support).

## Recent Changes

The following files were modified to fix this issue:

1. **igra-web-ui/src/services/api.js**
   - Added `credentials: 'include'` to fetch requests (for session cookies)
   - Removed old Bearer token authentication

2. **src/server/routes.rs**
   - Reorganized layer application order
   - Auth layer now properly applied

## Next Steps If Still Broken

If the issue persists after rebuilding:

1. Check if routes are actually registered:
   ```bash
   RUST_LOG=trace ./target/release/igra-cli server --host 0.0.0.0 --port 9000 2>&1 | grep "route"
   ```

2. Verify axum-login integration:
   ```bash
   # Check dependencies
   cargo tree | grep axum-login
   # Should show: axum-login v0.18.0
   ```

3. Test without authentication (temporarily):
   - Comment out `.layer(auth_layer)` in routes.rs
   - Rebuild and test if routes work
   - This isolates whether it's a routing issue or auth layer issue

4. Check for port conflicts:
   ```bash
   sudo lsof -i :9000
   # Should only show one igra-cli process
   ```

## Contact

If none of these steps work, please provide:
1. Full server startup logs (with RUST_LOG=debug)
2. Browser DevTools Network tab screenshot showing the failing request
3. Output of: `cargo --version` and `npm --version`
4. Output of: `git log -1 --oneline` (to confirm you have latest code)
