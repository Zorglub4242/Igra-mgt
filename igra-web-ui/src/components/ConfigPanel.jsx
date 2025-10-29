import { useState, useEffect } from 'react'
import { api } from '../services/api'
import AdminPanel from './AdminPanel'

export default function ConfigPanel({ user }) {
  const [activeTab, setActiveTab] = useState('environment')
  const [config, setConfig] = useState({})
  const [rpcTokens, setRpcTokens] = useState([])
  const [sslInfo, setSslInfo] = useState(null)
  const [systemInfo, setSystemInfo] = useState(null)
  const [versionInfo, setVersionInfo] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [searchTerm, setSearchTerm] = useState('')
  const [restarting, setRestarting] = useState(false)
  const [updating, setUpdating] = useState(false)
  const [actionStatus, setActionStatus] = useState(null)
  const [autoUpdateEnabled, setAutoUpdateEnabled] = useState(true)
  const [monitoringUrl, setMonitoringUrl] = useState('')
  const [editingMonitoringUrl, setEditingMonitoringUrl] = useState(false)
  const [tempMonitoringUrl, setTempMonitoringUrl] = useState('')
  const [editingKey, setEditingKey] = useState(null)
  const [editValue, setEditValue] = useState('')
  const [saving, setSaving] = useState(false)
  const [availableServices, setAvailableServices] = useState([])
  const [servicesLoading, setServicesLoading] = useState(false)
  const [serviceSearchTerm, setServiceSearchTerm] = useState('')
  const [categories, setCategories] = useState([])
  const [sortBy, setSortBy] = useState('name') // name, status, category
  const [sortDir, setSortDir] = useState('asc') // asc, desc
  const [statusFilter, setStatusFilter] = useState('all') // all, running, stopped, failed
  const [categoryFilter, setCategoryFilter] = useState('all') // all, or category id

  useEffect(() => {
    loadData()
    // Load preferences
    const autoUpdate = localStorage.getItem('auto_update_enabled')
    setAutoUpdateEnabled(autoUpdate !== 'false') // Default true

    const savedUrl = localStorage.getItem('monitoring_url') || 'https://grafana.igralabs.com/public-dashboards/5de66581390e434a823a2206237f793b'
    setMonitoringUrl(savedUrl)
    setTempMonitoringUrl(savedUrl)
  }, [])

  async function loadData() {
    try {
      const [configData, tokensData, sslData, sysData, verData] = await Promise.all([
        api.getConfig(),
        api.getRpcTokens(),
        api.getSslInfo(),
        api.getSystemInfo(),
        fetch('/api/version').then(r => r.json()).then(d => d.data).catch(() => null)
      ])

      setConfig(configData)
      setRpcTokens(tokensData)
      setSslInfo(sslData)
      setSystemInfo(sysData)
      setVersionInfo(verData)
      setError(null)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  async function handleRestartService() {
    if (!confirm('This will restart the igra-web-ui service. The page will reload after restart. Continue?')) {
      return
    }

    setRestarting(true)
    setActionStatus({ message: 'Restarting service...', success: null })

    try {
      const result = await api.restartService()
      setActionStatus({ message: result.message, success: result.success })

      if (result.success) {
        // Auto-reload after 5 seconds
        setTimeout(() => {
          window.location.reload()
        }, 5000)
      }
    } catch (err) {
      setActionStatus({ message: `Error: ${err.message}`, success: false })
    } finally {
      setRestarting(false)
    }
  }

  async function handleForceUpdate() {
    if (!confirm('This will update igra-cli to the latest version and restart the service. Continue?')) {
      return
    }

    setUpdating(true)
    setActionStatus({ message: 'Starting update...', success: null })

    try {
      const token = api.getToken()
      const response = await fetch('/api/update', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`
        }
      })

      const data = await response.json()

      if (data.success && data.data) {
        setActionStatus({ message: data.data.message, success: data.data.success })

        if (data.data.success) {
          // Auto-reload after 7 seconds
          setTimeout(() => {
            window.location.reload()
          }, 7000)
        }
      } else {
        setActionStatus({ message: data.error || 'Update failed', success: false })
        setUpdating(false)
      }
    } catch (err) {
      setActionStatus({ message: `Error: ${err.message}`, success: false })
      setUpdating(false)
    }
  }

  function copyToClipboard(text) {
    navigator.clipboard.writeText(text)
    alert('Copied to clipboard!')
  }

  function isSensitiveKey(key) {
    const sensitive = ['PASSWORD', 'SECRET', 'KEY', 'TOKEN']
    return sensitive.some(s => key.includes(s))
  }

  function maskValue(value) {
    if (value.length <= 8) return '****'
    return value.substring(0, 4) + '****' + value.substring(value.length - 4)
  }

  function handleAutoUpdateToggle(enabled) {
    setAutoUpdateEnabled(enabled)
    localStorage.setItem('auto_update_enabled', enabled ? 'true' : 'false')
    alert(`Auto-update ${enabled ? 'enabled' : 'disabled'}. ${enabled ? 'Update notifications will appear automatically.' : 'You can still manually update from this page.'}`)
  }

  function handleSaveMonitoringUrl() {
    localStorage.setItem('monitoring_url', tempMonitoringUrl)
    setMonitoringUrl(tempMonitoringUrl)
    setEditingMonitoringUrl(false)
    alert('Monitoring URL saved! Reload the Monitoring tab to see changes.')
  }

  function handleCancelMonitoringUrl() {
    setTempMonitoringUrl(monitoringUrl)
    setEditingMonitoringUrl(false)
  }

  function handleEditConfig(key, value) {
    setEditingKey(key)
    setEditValue(value)
  }

  function handleCancelEdit() {
    setEditingKey(null)
    setEditValue('')
  }

  async function handleSaveConfig(key) {
    setSaving(true)
    try {
      // Note: This would require a backend endpoint to update .env
      // For now, we'll just show a message
      alert(`Saving ${key} is not yet implemented in the backend. This would require an endpoint to update the .env file securely.`)
      setEditingKey(null)
      setEditValue('')
    } catch (err) {
      alert(`Error: ${err.message}`)
    } finally {
      setSaving(false)
    }
  }

  async function loadAvailableServices() {
    setServicesLoading(true)
    try {
      const [services, cats] = await Promise.all([
        api.getAvailableSystemServices(),
        api.getCategories()
      ])
      setAvailableServices(services)
      setCategories(cats)
    } catch (err) {
      console.error('Failed to load services:', err)
      alert(`Error loading services: ${err.message}`)
    } finally {
      setServicesLoading(false)
    }
  }

  async function handleToggleTracking(serviceName, currentlyTracked) {
    try {
      if (currentlyTracked) {
        await api.removeTrackedService(serviceName)
      } else {
        await api.updateTrackedService(serviceName, {
          category: 'uncategorized',
          display_name: serviceName.replace('.service', ''),
          metrics_enabled: true,
          plugin: null
        })
      }
      await loadAvailableServices()
    } catch (err) {
      alert(`Error: ${err.message}`)
    }
  }

  async function handleCategoryChange(serviceName, categoryId) {
    try {
      const service = availableServices.find(s => s.name === serviceName)
      if (service && service.is_tracked) {
        await api.updateTrackedService(serviceName, {
          category: categoryId,
          display_name: service.display_name,
          metrics_enabled: true,
          plugin: null
        })
        await loadAvailableServices()
      }
    } catch (err) {
      alert(`Error: ${err.message}`)
    }
  }

  function renderEnvironmentTab() {
    const entries = Object.entries(config)
      .filter(([key]) => key.toLowerCase().includes(searchTerm.toLowerCase()))
      .sort(([a], [b]) => a.localeCompare(b))

    // Group by category
    const categories = {
      'Network & Chain': entries.filter(([k]) => k.includes('NETWORK') || k.includes('CHAIN') || k.includes('IGRA_')),
      'Kaspad Configuration': entries.filter(([k]) => k.includes('KASPAD_') || k.includes('KASPA_')),
      'Wallet Configuration': entries.filter(([k]) => k.includes('WALLET') || k.includes('W0_') || k.includes('W1_') || k.includes('W2_') || k.includes('W3_') || k.includes('W4_')),
      'RPC & Access': entries.filter(([k]) => k.includes('RPC_') || k.includes('TOKEN_')),
      'SSL & Domain': entries.filter(([k]) => k.includes('DOMAIN') || k.includes('OVH_') || k.includes('EMAIL')),
      'Monitoring & System': entries.filter(([k]) => k.includes('NODE_ID') || k.includes('MONITORING')),
      'Other': entries.filter(([k]) => {
        const included = ['NETWORK', 'CHAIN', 'IGRA_', 'KASPAD_', 'KASPA_', 'WALLET', 'W0_', 'W1_', 'W2_', 'W3_', 'W4_', 'RPC_', 'TOKEN_', 'DOMAIN', 'OVH_', 'EMAIL', 'NODE_ID', 'MONITORING']
        return !included.some(prefix => k.includes(prefix))
      })
    }

    return (
      <div>
        <div style={{ marginBottom: '1.5rem' }}>
          <input
            type="text"
            className="log-search"
            placeholder="Search configuration..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            style={{ width: '100%', maxWidth: '400px' }}
          />
        </div>

        {Object.entries(categories).map(([category, items]) => {
          if (items.length === 0) return null

          return (
            <div key={category} style={{ marginBottom: '2rem' }}>
              <h3 style={{ color: '#818cf8', marginBottom: '1rem', fontSize: '1.125rem' }}>
                {category}
              </h3>
              <table className="table">
                <thead>
                  <tr>
                    <th style={{ width: '30%' }}>Key</th>
                    <th>Value</th>
                    <th style={{ width: '120px' }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {items.map(([key, value]) => {
                    const sensitive = isSensitiveKey(key)
                    const isEditing = editingKey === key
                    const displayValue = sensitive ? maskValue(value) : (value.length > 60 ? value.substring(0, 57) + '...' : value)

                    return (
                      <tr key={key}>
                        <td>
                          <code style={{ fontSize: '0.875rem', color: '#e2e8f0' }}>{key}</code>
                        </td>
                        <td>
                          {isEditing ? (
                            <input
                              type={sensitive ? "password" : "text"}
                              className="log-search"
                              value={editValue}
                              onChange={(e) => setEditValue(e.target.value)}
                              style={{ width: '100%', fontSize: '0.875rem', fontFamily: 'monospace' }}
                              autoFocus
                            />
                          ) : (
                            <span style={{ fontFamily: 'monospace', fontSize: '0.875rem', color: sensitive ? '#9ca3af' : '#cbd5e1' }}>
                              {displayValue || <em style={{ color: '#64748b' }}>(empty)</em>}
                            </span>
                          )}
                        </td>
                        <td>
                          {isEditing ? (
                            <div style={{ display: 'flex', gap: '0.25rem' }}>
                              <button
                                className="btn btn-sm btn-success"
                                onClick={() => handleSaveConfig(key)}
                                disabled={saving}
                                style={{ padding: '0.25rem 0.5rem', fontSize: '0.75rem' }}
                              >
                                💾
                              </button>
                              <button
                                className="btn btn-sm"
                                onClick={handleCancelEdit}
                                disabled={saving}
                                style={{ padding: '0.25rem 0.5rem', fontSize: '0.75rem' }}
                              >
                                ✖
                              </button>
                            </div>
                          ) : (
                            <button
                              className="btn btn-sm"
                              onClick={() => handleEditConfig(key, value)}
                              style={{ padding: '0.25rem 0.5rem', fontSize: '0.75rem' }}
                            >
                              ✏️ Edit
                            </button>
                          )}
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </div>
          )
        })}
      </div>
    )
  }

  function renderRpcTokensTab() {
    const domain = sslInfo?.domain || config.DOMAIN || 'localhost'
    const hasSSL = sslInfo?.domain ? true : false
    const protocol = hasSSL ? 'https' : 'http'
    const port = hasSSL ? '9443' : '8545'

    return (
      <div>
        <div className="card" style={{ marginBottom: '1.5rem', background: '#0f172a' }}>
          <div style={{ padding: '1rem' }}>
            <div style={{ fontSize: '0.875rem', color: '#94a3b8', marginBottom: '0.25rem' }}>
              RPC Domain
            </div>
            <div style={{ fontSize: '1.125rem', fontWeight: 'bold', color: '#e2e8f0', fontFamily: 'monospace' }}>
              {domain}
            </div>
            <div style={{ fontSize: '0.75rem', color: '#64748b', marginTop: '0.5rem' }}>
              {protocol}://{domain}:{port}/[TOKEN]/
            </div>
          </div>
        </div>

        <table className="table">
          <thead>
            <tr>
              <th>Token #</th>
              <th>Value</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {rpcTokens.map((item) => {
              const hasToken = item.token !== null && item.token !== undefined

              return (
                <tr key={item.index}>
                  <td>
                    <strong>TOKEN_{String(item.index).padStart(2, '0')}</strong>
                  </td>
                  <td>
                    <span style={{ fontFamily: 'monospace', fontSize: '0.875rem', color: hasToken ? '#cbd5e1' : '#64748b', wordBreak: 'break-all' }}>
                      {hasToken ? item.token : '<not set>'}
                    </span>
                  </td>
                  <td>
                    <span className={`badge ${hasToken ? 'badge-success' : 'badge-danger'}`}>
                      {hasToken ? '✓ Set' : '✗ Missing'}
                    </span>
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    )
  }

  function renderSystemTab() {
    return (
      <div>
        {/* Application Settings */}
        <div className="card" style={{ marginBottom: '1.5rem' }}>
          <h3 className="card-title">Application Settings</h3>

          {/* Auto-Update Toggle */}
          <div style={{ marginBottom: '1.5rem', padding: '1rem', background: '#0f172a', borderRadius: '0.5rem', border: '1px solid #334155' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div style={{ flex: 1 }}>
                <div style={{ fontSize: '1rem', fontWeight: 'bold', color: '#e2e8f0', marginBottom: '0.25rem' }}>
                  Automatic Update Notifications
                </div>
                <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>
                  Show banner when new version is available (checks every 6 hours)
                </div>
              </div>
              <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={autoUpdateEnabled}
                  onChange={(e) => handleAutoUpdateToggle(e.target.checked)}
                  style={{ width: '20px', height: '20px', cursor: 'pointer' }}
                />
                <span style={{ color: '#e2e8f0', fontSize: '0.875rem' }}>
                  {autoUpdateEnabled ? 'Enabled' : 'Disabled'}
                </span>
              </label>
            </div>
          </div>

          {/* Monitoring URL Configuration */}
          <div style={{ padding: '1rem', background: '#0f172a', borderRadius: '0.5rem', border: '1px solid #334155' }}>
            <div style={{ fontSize: '1rem', fontWeight: 'bold', color: '#e2e8f0', marginBottom: '0.5rem' }}>
              Monitoring Dashboard URL
            </div>
            <div style={{ fontSize: '0.875rem', color: '#94a3b8', marginBottom: '1rem' }}>
              Configure the URL for the embedded monitoring dashboard (Grafana, etc.)
            </div>

            {editingMonitoringUrl ? (
              <div>
                <input
                  type="text"
                  className="log-search"
                  value={tempMonitoringUrl}
                  onChange={(e) => setTempMonitoringUrl(e.target.value)}
                  placeholder="https://your-monitoring-url.com/dashboard"
                  style={{ width: '100%', marginBottom: '0.5rem' }}
                />
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                  <button className="btn btn-sm btn-success" onClick={handleSaveMonitoringUrl}>
                    💾 Save
                  </button>
                  <button className="btn btn-sm" onClick={handleCancelMonitoringUrl}>
                    ✖ Cancel
                  </button>
                </div>
              </div>
            ) : (
              <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
                <div style={{ flex: 1, fontFamily: 'monospace', fontSize: '0.875rem', color: '#cbd5e1', padding: '0.5rem', background: '#1e293b', borderRadius: '0.375rem', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {monitoringUrl}
                </div>
                <button className="btn btn-sm" onClick={() => setEditingMonitoringUrl(true)}>
                  ✏️ Edit
                </button>
              </div>
            )}
          </div>
        </div>

        {/* System Information */}
        {systemInfo && (
          <div className="card" style={{ marginBottom: '1.5rem' }}>
            <h3 className="card-title">System Information</h3>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1rem' }}>
              <div>
                <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>CPU</div>
                <div style={{ fontSize: '1rem', color: '#e2e8f0', marginTop: '0.25rem' }}>{systemInfo.cpu_model}</div>
              </div>
              <div>
                <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>Memory</div>
                <div style={{ fontSize: '1rem', color: '#e2e8f0', marginTop: '0.25rem' }}>{systemInfo.memory_total_gb?.toFixed(1)} GB</div>
              </div>
              <div>
                <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>Disk</div>
                <div style={{ fontSize: '1rem', color: '#e2e8f0', marginTop: '0.25rem' }}>
                  {systemInfo.disk_free_gb?.toFixed(1)} / {systemInfo.disk_total_gb?.toFixed(1)} GB
                </div>
              </div>
              <div>
                <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>OS</div>
                <div style={{ fontSize: '1rem', color: '#e2e8f0', marginTop: '0.25rem' }}>{systemInfo.os_name}</div>
              </div>
            </div>
          </div>
        )}

        {/* Version Information */}
        {versionInfo && (
          <div className="card" style={{ marginBottom: '1.5rem' }}>
            <h3 className="card-title">Version Information</h3>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1rem', marginBottom: '1rem' }}>
              <div>
                <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>Current Version</div>
                <div style={{ fontSize: '1.25rem', fontWeight: 'bold', color: '#e2e8f0', marginTop: '0.25rem' }}>
                  v{versionInfo.current_version}
                </div>
              </div>
              {versionInfo.update_available && (
                <div>
                  <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>Latest Version</div>
                  <div style={{ fontSize: '1.25rem', fontWeight: 'bold', color: '#10b981', marginTop: '0.25rem' }}>
                    v{versionInfo.latest_version}
                  </div>
                </div>
              )}
              <div>
                <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>Update Status</div>
                <div style={{ marginTop: '0.25rem' }}>
                  <span className={`badge ${versionInfo.update_available ? 'badge-warning' : 'badge-success'}`}>
                    {versionInfo.update_available ? '⚠️ Update Available' : '✓ Up to date'}
                  </span>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Service Actions */}
        <div className="card">
          <h3 className="card-title">Service Actions</h3>

          {actionStatus && (
            <div style={{
              padding: '1rem',
              marginBottom: '1rem',
              borderRadius: '0.5rem',
              background: actionStatus.success === false ? 'rgba(239, 68, 68, 0.1)' : actionStatus.success === true ? 'rgba(16, 185, 129, 0.1)' : 'rgba(99, 102, 241, 0.1)',
              border: `1px solid ${actionStatus.success === false ? '#ef4444' : actionStatus.success === true ? '#10b981' : '#6366f1'}`,
              color: actionStatus.success === false ? '#fca5a5' : actionStatus.success === true ? '#6ee7b7' : '#a5b4fc'
            }}>
              {actionStatus.message}
            </div>
          )}

          <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap' }}>
            <button
              className="btn btn-warning"
              onClick={handleRestartService}
              disabled={restarting || updating}
              style={{ minWidth: '160px' }}
            >
              {restarting ? '⏳ Restarting...' : '🔄 Restart Service'}
            </button>

            <button
              className="btn"
              onClick={handleForceUpdate}
              disabled={updating || restarting || !versionInfo?.update_available}
              style={{ minWidth: '160px', background: versionInfo?.update_available ? '#10b981' : undefined }}
            >
              {updating ? '⏳ Updating...' : '⬆️ Force Update'}
            </button>
          </div>

          <div style={{ marginTop: '1rem', padding: '1rem', background: 'rgba(99, 102, 241, 0.1)', borderRadius: '0.5rem', border: '1px solid #6366f1' }}>
            <p style={{ margin: 0, color: '#818cf8', fontSize: '0.875rem' }}>
              <strong>⚠️ Warning:</strong> These actions will temporarily interrupt service. Restart Service will reload the web UI.
              Force Update will download the latest version and restart the service. The page will automatically reload when ready.
            </p>
          </div>
        </div>
      </div>
    )
  }

  function handleSort(column) {
    if (sortBy === column) {
      setSortDir(sortDir === 'asc' ? 'desc' : 'asc')
    } else {
      setSortBy(column)
      setSortDir('asc')
    }
  }

  function renderSystemServicesTab() {
    // Filter by search term
    let filteredServices = availableServices.filter(s =>
      s.name.toLowerCase().includes(serviceSearchTerm.toLowerCase()) ||
      s.display_name.toLowerCase().includes(serviceSearchTerm.toLowerCase())
    )

    // Filter by status
    if (statusFilter !== 'all') {
      filteredServices = filteredServices.filter(s => s.status === statusFilter)
    }

    // Filter by category
    if (categoryFilter !== 'all') {
      if (categoryFilter === 'uncategorized') {
        filteredServices = filteredServices.filter(s => !s.category || s.category === '')
      } else {
        filteredServices = filteredServices.filter(s => s.category === categoryFilter)
      }
    }

    // Sort services
    filteredServices.sort((a, b) => {
      let compareA, compareB

      if (sortBy === 'name') {
        compareA = a.display_name.toLowerCase()
        compareB = b.display_name.toLowerCase()
      } else if (sortBy === 'status') {
        compareA = a.status
        compareB = b.status
      } else if (sortBy === 'category') {
        compareA = a.category || 'zzz' // Put uncategorized at end
        compareB = b.category || 'zzz'
      }

      if (compareA < compareB) return sortDir === 'asc' ? -1 : 1
      if (compareA > compareB) return sortDir === 'asc' ? 1 : -1
      return 0
    })

    return (
      <div>
        <h3 style={{ marginBottom: '1rem' }}>System Services Management</h3>
        <p style={{ color: '#94a3b8', marginBottom: '1.5rem', fontSize: '0.875rem' }}>
          Select which system services you want to track and manage. Only tracked services will appear in the main services panel.
        </p>

        {/* Search and filters */}
        <div style={{ display: 'flex', gap: '1rem', marginBottom: '1rem', flexWrap: 'wrap' }}>
          <input
            type="text"
            placeholder="Search services..."
            value={serviceSearchTerm}
            onChange={(e) => setServiceSearchTerm(e.target.value)}
            style={{
              flex: '1',
              minWidth: '200px',
              padding: '0.5rem',
              borderRadius: '0.375rem',
              background: '#0f172a',
              border: '1px solid #334155',
              color: '#e2e8f0'
            }}
          />

          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value)}
            style={{
              padding: '0.5rem',
              borderRadius: '0.375rem',
              background: '#0f172a',
              border: '1px solid #334155',
              color: '#e2e8f0',
              minWidth: '120px'
            }}
          >
            <option value="all">All Statuses</option>
            <option value="running">Running</option>
            <option value="stopped">Stopped</option>
            <option value="failed">Failed</option>
          </select>

          <select
            value={categoryFilter}
            onChange={(e) => setCategoryFilter(e.target.value)}
            style={{
              padding: '0.5rem',
              borderRadius: '0.375rem',
              background: '#0f172a',
              border: '1px solid #334155',
              color: '#e2e8f0',
              minWidth: '150px'
            }}
          >
            <option value="all">All Categories</option>
            <option value="uncategorized">Uncategorized</option>
            {categories.map(cat => (
              <option key={cat.id} value={cat.id}>
                {cat.icon} {cat.name}
              </option>
            ))}
          </select>
        </div>

        {/* Services table */}
        <table className="table">
          <thead>
            <tr>
              <th style={{ width: '5%' }}>Track</th>
              <th
                style={{ width: '25%', cursor: 'pointer', userSelect: 'none' }}
                onClick={() => handleSort('name')}
              >
                Service Name {sortBy === 'name' && (sortDir === 'asc' ? '▲' : '▼')}
              </th>
              <th style={{ width: '35%' }}>Description</th>
              <th
                style={{ width: '15%', cursor: 'pointer', userSelect: 'none' }}
                onClick={() => handleSort('status')}
              >
                Status {sortBy === 'status' && (sortDir === 'asc' ? '▲' : '▼')}
              </th>
              <th
                style={{ width: '20%', cursor: 'pointer', userSelect: 'none' }}
                onClick={() => handleSort('category')}
              >
                Category {sortBy === 'category' && (sortDir === 'asc' ? '▲' : '▼')}
              </th>
            </tr>
          </thead>
          <tbody>
            {servicesLoading ? (
              <tr>
                <td colSpan="5" style={{ textAlign: 'center', padding: '2rem' }}>
                  Loading services...
                </td>
              </tr>
            ) : filteredServices.length === 0 ? (
              <tr>
                <td colSpan="5" style={{ textAlign: 'center', padding: '2rem', color: '#94a3b8' }}>
                  No services found
                </td>
              </tr>
            ) : (
              filteredServices.map(service => (
                <tr key={service.name}>
                  <td>
                    <input
                      type="checkbox"
                      checked={service.is_tracked}
                      onChange={() => handleToggleTracking(service.name, service.is_tracked)}
                      style={{ cursor: 'pointer', width: '18px', height: '18px' }}
                    />
                  </td>
                  <td>
                    <strong>{service.display_name}</strong>
                    <div style={{ fontSize: '0.75rem', color: '#64748b' }}>
                      {service.name}
                    </div>
                  </td>
                  <td style={{ fontSize: '0.875rem', color: '#94a3b8' }}>
                    {service.description || '-'}
                  </td>
                  <td>
                    <span className={`badge badge-${
                      service.status === 'running' ? 'success' : 'danger'
                    }`}>
                      {service.status}
                    </span>
                  </td>
                  <td>
                    {service.is_tracked ? (
                      <select
                        value={service.category || ''}
                        onChange={(e) => handleCategoryChange(service.name, e.target.value)}
                        style={{
                          padding: '0.25rem 0.5rem',
                          borderRadius: '0.25rem',
                          background: '#0f172a',
                          border: '1px solid #334155',
                          color: '#e2e8f0',
                          fontSize: '0.875rem',
                          width: '100%'
                        }}
                      >
                        <option value="">Uncategorized</option>
                        {categories.map(cat => (
                          <option key={cat.id} value={cat.id}>
                            {cat.icon} {cat.name}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <span style={{ color: '#64748b', fontSize: '0.875rem' }}>-</span>
                    )}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>

        <button
          className="btn"
          onClick={loadAvailableServices}
          style={{ marginTop: '1rem' }}
          disabled={servicesLoading}
        >
          🔄 Refresh
        </button>
      </div>
    )
  }

  if (loading) {
    return <div className="loading">Loading configuration...</div>
  }

  if (error) {
    return <div className="error">Error: {error}</div>
  }

  return (
    <div className="card">
      <div className="card-header">
        <h2 className="card-title">⚙️ Settings & Configuration</h2>
      </div>

      {/* Tab Navigation */}
      <div style={{ borderBottom: '2px solid #334155', marginBottom: '1.5rem' }}>
        <div style={{ display: 'flex', gap: '0.5rem', padding: '0 1.5rem' }}>
          <button
            className={`tab ${activeTab === 'environment' ? 'active' : ''}`}
            onClick={() => setActiveTab('environment')}
            style={{
              background: 'transparent',
              border: 'none',
              color: activeTab === 'environment' ? '#818cf8' : '#94a3b8',
              padding: '0.75rem 1.25rem',
              cursor: 'pointer',
              fontSize: '1rem',
              fontWeight: '500',
              borderBottom: activeTab === 'environment' ? '3px solid #818cf8' : '3px solid transparent',
              transition: 'all 0.2s'
            }}
          >
            Environment
          </button>
          <button
            className={`tab ${activeTab === 'rpc' ? 'active' : ''}`}
            onClick={() => setActiveTab('rpc')}
            style={{
              background: 'transparent',
              border: 'none',
              color: activeTab === 'rpc' ? '#818cf8' : '#94a3b8',
              padding: '0.75rem 1.25rem',
              cursor: 'pointer',
              fontSize: '1rem',
              fontWeight: '500',
              borderBottom: activeTab === 'rpc' ? '3px solid #818cf8' : '3px solid transparent',
              transition: 'all 0.2s'
            }}
          >
            RPC Tokens
          </button>
          <button
            className={`tab ${activeTab === 'system' ? 'active' : ''}`}
            onClick={() => setActiveTab('system')}
            style={{
              background: 'transparent',
              border: 'none',
              color: activeTab === 'system' ? '#818cf8' : '#94a3b8',
              padding: '0.75rem 1.25rem',
              cursor: 'pointer',
              fontSize: '1rem',
              fontWeight: '500',
              borderBottom: activeTab === 'system' ? '3px solid #818cf8' : '3px solid transparent',
              transition: 'all 0.2s'
            }}
          >
            System
          </button>
          <button
            className={`tab ${activeTab === 'system-services' ? 'active' : ''}`}
            onClick={() => {
              setActiveTab('system-services')
              loadAvailableServices()
            }}
            style={{
              background: 'transparent',
              border: 'none',
              color: activeTab === 'system-services' ? '#818cf8' : '#94a3b8',
              padding: '0.75rem 1.25rem',
              cursor: 'pointer',
              fontSize: '1rem',
              fontWeight: '500',
              borderBottom: activeTab === 'system-services' ? '3px solid #818cf8' : '3px solid transparent',
              transition: 'all 0.2s'
            }}
          >
            ⚙️ System Services
          </button>
          {user?.roles?.includes('admin') && (
            <button
              className={`tab ${activeTab === 'admin' ? 'active' : ''}`}
              onClick={() => setActiveTab('admin')}
              style={{
                background: 'transparent',
                border: 'none',
                color: activeTab === 'admin' ? '#818cf8' : '#94a3b8',
                padding: '0.75rem 1.25rem',
                cursor: 'pointer',
                fontSize: '1rem',
                fontWeight: '500',
                borderBottom: activeTab === 'admin' ? '3px solid #818cf8' : '3px solid transparent',
                transition: 'all 0.2s'
              }}
            >
              🔐 Admin
            </button>
          )}
        </div>
      </div>

      {/* Tab Content */}
      <div style={{ padding: '0 1.5rem 1.5rem' }}>
        {activeTab === 'environment' && renderEnvironmentTab()}
        {activeTab === 'rpc' && renderRpcTokensTab()}
        {activeTab === 'system' && renderSystemTab()}
        {activeTab === 'system-services' && renderSystemServicesTab()}
        {activeTab === 'admin' && <AdminPanel user={user} />}
      </div>
    </div>
  )
}
