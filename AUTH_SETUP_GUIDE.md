# Authentication Setup Guide

This guide explains how to set up and use the multi-user authentication system in igra-cli.

## Overview

igra-cli now includes a comprehensive authentication and security system with:
- **Session-based authentication** using axum-login
- **Role-based access control** (RBAC) with three roles: Admin, Operator, Viewer
- **IP allowlisting** for network security
- **Audit logging** for compliance and security monitoring
- **File-based user storage** (YAML format)

## Quick Start

### 1. First Launch

When you start the server for the first time, a default admin user is automatically created:

```bash
igra-cli server --host 0.0.0.0 --port 9000
```

Output:
```
⚠️  No users found. Creating default admin user...
   Default password: admin
   ⚠️  CHANGE THIS PASSWORD IMMEDIATELY!
✓ Default admin user created
```

### 2. Login to Web UI

Navigate to http://localhost:9000 in your browser. You'll see the login page.

**Default credentials:**
- Username: `admin`
- Password: `admin`

**⚠️ IMPORTANT:** Change the default password immediately after first login!

### 3. Change Default Password

After logging in, use the CLI to change the admin password:

```bash
igra-cli user reset-password admin
# Enter new password when prompted
```

## User Management

### List Users

```bash
igra-cli user list
```

### Add a New User

```bash
# Interactive mode (prompts for password)
igra-cli user add <username> --roles <role1,role2>

# Examples
igra-cli user add operator1 --roles operator
igra-cli user add viewer1 --roles viewer
igra-cli user add poweruser --roles admin,operator
```

### Remove a User

```bash
igra-cli user remove <username>
```

### Reset User Password

```bash
igra-cli user reset-password <username>
# Enter new password when prompted
```

### Enable/Disable Users

```bash
igra-cli user set-enabled <username> true
igra-cli user set-enabled <username> false
```

### View User Details

```bash
igra-cli user show <username>
```

## Roles and Permissions

### Admin
- Full system access
- Can manage users
- Can configure settings
- Can control services
- Can view all data

### Operator
- Can control services (start/stop/restart)
- Can view logs and metrics
- Cannot manage users
- Cannot change system configuration

### Viewer
- Read-only access
- Can view services, logs, and metrics
- Cannot control services
- Cannot manage users or configuration

## IP Security

### View Current Security Configuration

```bash
igra-cli security show
```

### Add Allowed IP/Network

```bash
# Allow a single IP
igra-cli security ip add 192.168.1.100/32

# Allow a network
igra-cli security ip add 192.168.1.0/24

# Allow all (not recommended for production)
igra-cli security ip add 0.0.0.0/0
```

### Remove Allowed IP/Network

```bash
igra-cli security ip remove 192.168.1.100/32
```

### Proxy Configuration

If igra-cli is behind a reverse proxy (e.g., nginx, traefik), configure proxy settings:

Edit `~/.config/igra-cli/security.yaml`:

```yaml
allowed_networks:
  - "0.0.0.0/0"
trust_proxy: true
proxy_header: "X-Real-IP"  # or "X-Forwarded-For"
```

## Audit Logging

### View Audit Logs

```bash
# Show recent audit logs
igra-cli audit show

# Show specific number of entries
igra-cli audit show --limit 100
```

### Export Audit Logs

```bash
igra-cli audit export --output audit-$(date +%Y%m%d).json
```

### Clear Audit Logs

```bash
igra-cli audit clear
```

## Configuration Files

All authentication data is stored in `~/.config/igra-cli/`:

```
~/.config/igra-cli/
├── users.yaml       # User accounts and passwords
├── security.yaml    # IP allowlist and proxy settings
└── audit.jsonl      # Audit log (JSON Lines format)
```

### Example users.yaml

```yaml
users:
  - username: admin
    password_hash: "$argon2id$v=19$m=19456,t=2,p=1$..."
    roles:
      - Admin
    enabled: true
  - username: operator1
    password_hash: "$argon2id$v=19$m=19456,t=2,p=1$..."
    roles:
      - Operator
    enabled: true
```

### Example security.yaml

```yaml
allowed_networks:
  - "192.168.1.0/24"
  - "10.0.0.0/8"
trust_proxy: false
proxy_header: "X-Real-IP"
```

## API Authentication

### Session Management

After login, a session cookie is automatically set (HttpOnly, valid for 24 hours). All subsequent API requests will include this cookie.

### Logout

To invalidate the session:

```bash
curl -X POST http://localhost:9000/api/auth/logout
```

Or click "Logout" in the web UI.

### Check Session Status

```bash
curl http://localhost:9000/api/auth/session
```

Response:
```json
{
  "authenticated": true,
  "username": "admin",
  "roles": ["Admin"]
}
```

## Security Best Practices

1. **Change default password immediately** after first login
2. **Use strong passwords** (minimum 8 characters, mix of letters/numbers/symbols)
3. **Configure IP allowlist** to restrict access to trusted networks
4. **Enable HTTPS** in production (use `--tls-cert` and `--tls-key` flags)
5. **Review audit logs regularly** for suspicious activity
6. **Follow principle of least privilege** - assign minimal required roles
7. **Disable unused accounts** instead of deleting them (preserves audit trail)
8. **Backup user and security configuration files** regularly

## Troubleshooting

### Locked Out (Forgot Password)

If you forget the admin password, you can manually reset it:

1. Stop the server
2. Delete the users.yaml file:
   ```bash
   rm ~/.config/igra-cli/users.yaml
   ```
3. Restart the server - a new default admin user will be created
4. Login with admin/admin and change the password

### Access Denied (IP Blocked)

If you're blocked by the IP allowlist:

1. Stop the server
2. Edit `~/.config/igra-cli/security.yaml`
3. Add your IP address to `allowed_networks`
4. Restart the server

Or temporarily allow all IPs:
```yaml
allowed_networks:
  - "0.0.0.0/0"
```

### Session Expired

Sessions expire after 24 hours of inactivity. Simply log in again.

### Behind Reverse Proxy (Real IP Not Detected)

Configure the proxy settings in security.yaml:

```yaml
trust_proxy: true
proxy_header: "X-Forwarded-For"  # or "X-Real-IP", depending on your proxy
```

## Web UI Features

The web interface includes:

- **Login Page** - Username/password authentication with session cookies
- **User Management** (Admin only) - Add, edit, disable users
- **Security Settings** (Admin only) - Configure IP allowlist
- **Audit Logs** (Admin only) - View and export audit trail
- **Logout Button** - Visible in top-right corner when authenticated

## Upgrade Notes

### From Previous Versions (No Authentication)

If you're upgrading from a version without authentication:

1. **Backup your data**
2. **Update to the latest version**
3. **First launch will create default admin user**
4. **Configure IP allowlist** if needed
5. **Create additional user accounts** as required

All existing functionality remains unchanged - authentication is added as a layer on top.

## Related Documentation

- `AUTH_IMPLEMENTATION_STATUS.md` - Implementation details and status
- `README.md` - General igra-cli documentation
- `CONTRIBUTING.md` - Contributing guidelines

## Support

For issues or questions:
- Check audit logs for authentication failures
- Review security.yaml for IP allowlist issues
- Enable debug logging: `RUST_LOG=debug igra-cli server`

---

**Security Note:** The authentication system uses industry-standard Argon2id password hashing and session-based authentication. However, always use HTTPS in production to protect credentials in transit.
