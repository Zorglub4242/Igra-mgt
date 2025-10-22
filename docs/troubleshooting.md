# Troubleshooting Guide

Common issues and solutions for `igra-cli`.

## Installation Issues

### Binary not found after installation

**Symptom:** `igra-cli: command not found`

**Solutions:**
1. Verify binary is in PATH:
   ```bash
   which igra-cli
   echo $PATH
   ```

2. Add `/usr/local/bin` to PATH if needed:
   ```bash
   echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bashrc
   source ~/.bashrc
   ```

3. Check binary permissions:
   ```bash
   ls -l /usr/local/bin/igra-cli
   sudo chmod +x /usr/local/bin/igra-cli
   ```

### Permission denied when running

**Symptom:** `Permission denied` error

**Solution:**
```bash
sudo chmod +x /usr/local/bin/igra-cli
```

## TUI Issues

### TUI won't start / crashes immediately

**Symptom:** Terminal closes or shows error on launch

**Solutions:**
1. Check Docker is running:
   ```bash
   docker ps
   ```

2. Verify Docker socket permissions:
   ```bash
   ls -l /var/run/docker.sock
   sudo usermod -aG docker $USER
   # Log out and back in for group change to take effect
   ```

3. Run with debug logging:
   ```bash
   RUST_LOG=debug igra-cli
   ```

### Configuration not loading

**Symptom:** Config screen shows empty or errors

**Solutions:**
1. Verify you're in the IGRA Orchestra directory:
   ```bash
   cd ~/igra-orchestra-public
   igra-cli
   ```

2. Check .env file exists:
   ```bash
   ls -l .env
   ```

3. Check file permissions:
   ```bash
   chmod 600 .env
   ```

### Services not showing in TUI

**Symptom:** Services screen is empty

**Solutions:**
1. Verify Docker containers are running:
   ```bash
   docker ps
   ```

2. Check Docker socket connection:
   ```bash
   docker info
   ```

3. Restart Docker daemon:
   ```bash
   sudo systemctl restart docker
   ```

### Wallet addresses not displaying

**Symptom:** Wallet screen shows errors or no addresses

**Solutions:**
1. Check wallet key files exist:
   ```bash
   ls -l keys/keys.kaswallet-*.json
   ```

2. Verify file permissions:
   ```bash
   chmod 600 keys/keys.kaswallet-*.json
   ```

3. Validate JSON format:
   ```bash
   cat keys/keys.kaswallet-0.json | jq .
   ```

## Web UI Issues

### Web UI not loading

**Symptom:** Browser shows connection error

**Solutions:**
1. Verify server is running:
   ```bash
   ps aux | grep igra-cli
   ```

2. Check server is listening on correct port:
   ```bash
   netstat -tlnp | grep 3000
   # or
   ss -tlnp | grep 3000
   ```

3. Test with curl:
   ```bash
   curl http://localhost:3000/api/health
   ```

4. Check firewall rules:
   ```bash
   sudo ufw status
   sudo ufw allow 3000/tcp
   ```

### Authentication fails

**Symptom:** Login rejected with correct token

**Solutions:**
1. Verify `IGRA_WEB_TOKEN` is set:
   ```bash
   echo $IGRA_WEB_TOKEN
   ```

2. Restart server with token:
   ```bash
   IGRA_WEB_TOKEN=your-token igra-cli serve
   ```

3. Check for special characters in token:
   - Use alphanumeric tokens only
   - Avoid quotes, spaces, or special shell characters

### API endpoints return errors

**Symptom:** 500 errors in browser console

**Solutions:**
1. Check server logs:
   ```bash
   # If running as systemd service
   sudo journalctl -u igra-web -f

   # If running in terminal
   # Check terminal output for errors
   ```

2. Verify Docker is accessible:
   ```bash
   docker ps
   ```

3. Test specific endpoint:
   ```bash
   curl -H "Authorization: Bearer your-token" http://localhost:3000/api/services
   ```

### CORS errors

**Symptom:** Browser shows CORS policy errors

**Solution:**
Ensure server is started with `--cors` flag:
```bash
IGRA_WEB_TOKEN=your-token igra-cli serve --cors
```

### Web UI shows empty data

**Symptom:** Panels load but show no services/wallets

**Solutions:**
1. Check Docker containers are running:
   ```bash
   docker ps
   ```

2. Verify .env configuration loaded:
   ```bash
   curl -H "Authorization: Bearer your-token" http://localhost:3000/api/config
   ```

3. Check browser console for API errors (F12)

## Docker Integration Issues

### Cannot connect to Docker daemon

**Symptom:** `Error: Cannot connect to the Docker daemon`

**Solutions:**
1. Start Docker daemon:
   ```bash
   sudo systemctl start docker
   ```

2. Check Docker socket:
   ```bash
   ls -l /var/run/docker.sock
   ```

3. Add user to docker group:
   ```bash
   sudo usermod -aG docker $USER
   newgrp docker
   ```

### Docker stats not updating

**Symptom:** Resource metrics frozen or zero

**Solutions:**
1. Restart `igra-cli`

2. Check Docker stats manually:
   ```bash
   docker stats --no-stream
   ```

3. Restart Docker daemon:
   ```bash
   sudo systemctl restart docker
   ```

## System Service Issues

### Systemd service won't start

**Symptom:** `systemctl start igra-web` fails

**Solutions:**
1. Check service status:
   ```bash
   sudo systemctl status igra-web
   ```

2. View detailed logs:
   ```bash
   sudo journalctl -u igra-web -n 50
   ```

3. Verify service file syntax:
   ```bash
   sudo systemd-analyze verify /etc/systemd/system/igra-web.service
   ```

4. Check WorkingDirectory exists:
   ```bash
   ls -ld /path/to/igra-orchestra-public
   ```

5. Verify binary path:
   ```bash
   which igra-cli
   ls -l /usr/local/bin/igra-cli
   ```

### Service starts but stops immediately

**Solutions:**
1. Check environment variables in service file

2. Verify IGRA_WEB_TOKEN is set

3. Test manual start:
   ```bash
   sudo -u your-username /usr/local/bin/igra-cli serve
   ```

## Performance Issues

### TUI is slow / laggy

**Solutions:**
1. Reduce update frequency (code change required)

2. Filter services:
   - Only run necessary containers
   - Use Docker Compose profiles

3. Check system resources:
   ```bash
   top
   htop
   ```

### Web UI slow to load

**Solutions:**
1. Check network latency

2. Reduce auto-refresh interval (edit React components)

3. Use production build (not dev server)

4. Monitor server resource usage:
   ```bash
   top -p $(pgrep igra-cli)
   ```

## Log Viewer Issues

### Logs not updating in TUI

**Solutions:**
1. Toggle live mode: press `l`

2. Restart TUI

3. Check Docker logs directly:
   ```bash
   docker logs -f service-name
   ```

### Log viewer shows "No logs available"

**Solutions:**
1. Verify service is running:
   ```bash
   docker ps | grep service-name
   ```

2. Check Docker logging driver:
   ```bash
   docker inspect service-name | grep LogConfig
   ```

## Build Issues

### Rust compilation errors

**Solutions:**
1. Update Rust:
   ```bash
   rustup update
   ```

2. Clean build:
   ```bash
   cargo clean
   cargo build
   ```

3. Check Rust version:
   ```bash
   rustc --version  # Should be 1.70+
   ```

### Web UI build fails

**Solutions:**
1. Clean npm cache:
   ```bash
   cd igra-web-ui
   rm -rf node_modules package-lock.json
   npm install
   npm run build
   ```

2. Check Node version:
   ```bash
   node --version  # Should be 18+
   ```

## Getting Help

If these solutions don't resolve your issue:

1. **Check existing issues**: [GitHub Issues](https://github.com/Zorglub4242/Igra-mgt/issues)

2. **Gather diagnostic info**:
   ```bash
   igra-cli --version
   docker --version
   uname -a
   ```

3. **Enable debug logging**:
   ```bash
   RUST_LOG=debug igra-cli
   ```

4. **Open a new issue** with:
   - Detailed description
   - Steps to reproduce
   - Error messages
   - System info
   - Logs (with sensitive info redacted)
