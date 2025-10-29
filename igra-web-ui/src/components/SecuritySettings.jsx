import { useState, useEffect } from 'react'
import { api } from '../services/api'
import './SecuritySettings.css'

export default function SecuritySettings() {
  const [config, setConfig] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [newNetwork, setNewNetwork] = useState('')
  const [adding, setAdding] = useState(false)

  useEffect(() => {
    loadConfig()
  }, [])

  async function loadConfig() {
    try {
      setLoading(true)
      setError('')
      const data = await api.getSecurityConfig()
      setConfig(data)
    } catch (err) {
      setError(err.message || 'Failed to load security configuration')
    } finally {
      setLoading(false)
    }
  }

  async function handleAddNetwork(e) {
    e.preventDefault()

    if (!newNetwork.trim()) {
      setError('Please enter a network address')
      return
    }

    try {
      setAdding(true)
      setError('')
      await api.addAllowedNetwork(newNetwork.trim())
      setNewNetwork('')
      await loadConfig()
    } catch (err) {
      setError(err.message || 'Failed to add network')
    } finally {
      setAdding(false)
    }
  }

  async function handleRemoveNetwork(network) {
    if (!confirm(`Remove "${network}" from allowed networks?`)) {
      return
    }

    try {
      setError('')
      await api.removeAllowedNetwork(network)
      await loadConfig()
    } catch (err) {
      setError(err.message || 'Failed to remove network')
    }
  }

  if (loading) {
    return <div className="loading">Loading security settings...</div>
  }

  return (
    <div className="security-settings">
      <div className="section">
        <h2>IP Allowlist</h2>
        <p className="section-description">
          Control which IP addresses and networks can access the API. Use CIDR notation (e.g., 192.168.1.0/24).
        </p>

        {error && (
          <div className="error-message">
            ⚠️ {error}
          </div>
        )}

        <form onSubmit={handleAddNetwork} className="add-network-form">
          <input
            type="text"
            value={newNetwork}
            onChange={e => setNewNetwork(e.target.value)}
            placeholder="e.g., 192.168.1.0/24 or 10.0.0.1/32"
            disabled={adding}
          />
          <button type="submit" className="btn-primary" disabled={adding}>
            {adding ? 'Adding...' : '➕ Add Network'}
          </button>
        </form>

        <div className="networks-list">
          {config?.allowed_ips && config.allowed_ips.length > 0 ? (
            <table>
              <thead>
                <tr>
                  <th>Network / IP Address</th>
                  <th>Description</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {config.allowed_ips.map(network => (
                  <tr key={network}>
                    <td>
                      <code className="network-code">{network}</code>
                    </td>
                    <td className="network-description">
                      {getNetworkDescription(network)}
                    </td>
                    <td>
                      <button
                        className="btn-small btn-danger"
                        onClick={() => handleRemoveNetwork(network)}
                        disabled={network === '0.0.0.0/0'}
                        title={network === '0.0.0.0/0' ? 'Cannot remove "allow all" while it\'s the only entry' : 'Remove'}
                      >
                        🗑️ Remove
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="empty-state">
              <p>No networks configured. Add your first allowed network above.</p>
            </div>
          )}
        </div>
      </div>

      <div className="section">
        <h2>Proxy Settings</h2>
        <p className="section-description">
          Configure how the server handles requests from reverse proxies (e.g., nginx, traefik).
        </p>

        <div className="setting-row">
          <div className="setting-info">
            <strong>Trust Proxy</strong>
            <p>Enable if behind a reverse proxy to extract real client IP</p>
          </div>
          <div className="setting-value">
            <span className={`status-badge ${config?.trust_proxy ? 'enabled' : 'disabled'}`}>
              {config?.trust_proxy ? '✓ Enabled' : '✗ Disabled'}
            </span>
          </div>
        </div>

        <div className="setting-row">
          <div className="setting-info">
            <strong>Proxy Header</strong>
            <p>Header used to extract real client IP</p>
          </div>
          <div className="setting-value">
            <code>{config?.proxy_header || 'X-Real-IP'}</code>
          </div>
        </div>

        <div className="info-box">
          <strong>ℹ️ Note:</strong> Proxy settings are configured in <code>~/.config/igra-cli/security.yaml</code>
          <br />
          Edit the file and restart the server to apply changes.
        </div>
      </div>

      <div className="section">
        <h2>Security Tips</h2>
        <ul className="tips-list">
          <li>🔒 Always use HTTPS in production environments</li>
          <li>🌐 Restrict access to trusted networks only</li>
          <li>🔄 Regularly review audit logs for suspicious activity</li>
          <li>👥 Follow principle of least privilege for user roles</li>
          <li>🔑 Enforce strong password policies</li>
        </ul>
      </div>
    </div>
  )
}

function getNetworkDescription(network) {
  if (network === '0.0.0.0/0') {
    return '⚠️ All IPs (not recommended for production)'
  }
  if (network.startsWith('192.168.')) {
    return 'Private network (RFC 1918)'
  }
  if (network.startsWith('10.')) {
    return 'Private network (RFC 1918)'
  }
  if (network.startsWith('172.')) {
    const second = parseInt(network.split('.')[1])
    if (second >= 16 && second <= 31) {
      return 'Private network (RFC 1918)'
    }
  }
  if (network.includes('/32')) {
    return 'Single IP address'
  }
  if (network.includes('/24')) {
    return 'Class C network (256 addresses)'
  }
  return 'Network range'
}
