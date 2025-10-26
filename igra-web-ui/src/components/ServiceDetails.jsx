import { useState, useEffect } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { api } from '../services/api'
import LogViewer from './LogViewer'

export default function ServiceDetails() {
  const { serviceName } = useParams()
  const navigate = useNavigate()
  const [details, setDetails] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [selectedTab, setSelectedTab] = useState('overview')
  const [actionLoading, setActionLoading] = useState(null)
  const [showLogs, setShowLogs] = useState(false)

  useEffect(() => {
    loadDetails()
    const interval = setInterval(loadDetails, 5000) // Refresh every 5s
    return () => clearInterval(interval)
  }, [serviceName])

  async function loadDetails() {
    try {
      const data = await api.getServiceDetails(serviceName)
      setDetails(data)
      setError(null)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  async function handleServiceAction(action) {
    const actionText = action === 'start' ? 'start' : action === 'stop' ? 'stop' : 'restart'
    if (!confirm(`Are you sure you want to ${actionText} ${serviceName}?`)) {
      return
    }

    setActionLoading(action)
    try {
      if (action === 'start') {
        await api.startService(serviceName)
      } else if (action === 'stop') {
        await api.stopService(serviceName)
      } else if (action === 'restart') {
        await api.restartService(serviceName)
      }
      await loadDetails()
    } catch (err) {
      alert(`Error: ${err.message}`)
    } finally {
      setActionLoading(null)
    }
  }

  if (loading) {
    return (
      <div style={{ padding: '1.5rem' }}>
        <div style={{ textAlign: 'center', padding: '3rem' }}>
          <div style={{ fontSize: '2rem', marginBottom: '1rem' }}>⏳</div>
          <div>Loading service details...</div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div style={{ padding: '1.5rem' }}>
        <div style={{ background: '#fef2f2', border: '1px solid #fecaca', borderRadius: '0.375rem', padding: '1rem', color: '#991b1b' }}>
          <strong>Error:</strong> {error}
        </div>
        <button onClick={() => navigate('/')} style={{ marginTop: '1rem' }} className="btn">
          ← Back to Services
        </button>
      </div>
    )
  }

  if (!details) {
    return null
  }

  const tabs = [
    { id: 'overview', label: 'Overview' },
    { id: 'metrics', label: 'Metrics' },
    { id: 'config', label: 'Configuration' },
    { id: 'storage', label: 'Storage' },
    { id: 'network', label: 'Network' },
  ]

  const formatBytes = (bytes) => {
    const KB = 1024
    const MB = KB * 1024
    const GB = MB * 1024

    if (bytes >= GB) return `${(bytes / GB).toFixed(2)} GB`
    if (bytes >= MB) return `${(bytes / MB).toFixed(2)} MB`
    if (bytes >= KB) return `${(bytes / KB).toFixed(2)} KB`
    return `${bytes} B`
  }

  const getStatusColor = (status) => {
    if (status.includes('Up')) return '#047857'
    if (status.includes('Exited')) return '#b91c1c'
    return '#9ca3af'
  }

  return (
    <div style={{ padding: '1.5rem' }}>
      <div style={{ marginBottom: '1.5rem' }}>
        <button onClick={() => navigate('/')} className="btn" style={{ marginBottom: '0.5rem' }}>
          ← Back to Services
        </button>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '0.5rem' }}>
          <h1 style={{ fontSize: '1.875rem', fontWeight: 'bold' }}>
            {details.name}
          </h1>
          <div style={{ display: 'flex', gap: '0.5rem' }}>
            <button
              onClick={() => handleServiceAction('start')}
              disabled={actionLoading === 'start' || details.state === 'running'}
              className="btn btn-success btn-sm"
            >
              {actionLoading === 'start' ? '⏳' : '▶️'} Start
            </button>
            <button
              onClick={() => handleServiceAction('stop')}
              disabled={actionLoading === 'stop' || details.state !== 'running'}
              className="btn btn-danger btn-sm"
            >
              {actionLoading === 'stop' ? '⏳' : '⏹️'} Stop
            </button>
            <button
              onClick={() => handleServiceAction('restart')}
              disabled={actionLoading === 'restart'}
              className="btn btn-warning btn-sm"
            >
              {actionLoading === 'restart' ? '⏳' : '🔄'} Restart
            </button>
            <button
              onClick={() => setShowLogs(true)}
              className="btn btn-sm"
            >
              📋 Logs
            </button>
            <button onClick={loadDetails} className="btn btn-sm">
              🔄 Refresh
            </button>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div style={{ borderBottom: '2px solid #334155', marginBottom: '1.5rem' }}>
        <div style={{ display: 'flex', gap: '1rem' }}>
          {tabs.map(tab => (
            <button
              key={tab.id}
              onClick={() => setSelectedTab(tab.id)}
              style={{
                padding: '0.75rem 1rem',
                border: 'none',
                background: 'none',
                borderBottom: selectedTab === tab.id ? '3px solid #3b82f6' : '3px solid transparent',
                color: selectedTab === tab.id ? '#3b82f6' : '#94a3b8',
                fontWeight: selectedTab === tab.id ? 'bold' : 'normal',
                cursor: 'pointer',
                transition: 'all 0.2s'
              }}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Tab Content */}
      <div>
        {selectedTab === 'overview' && (
          <div>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1.5rem', marginBottom: '1.5rem' }}>
              <div style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155' }}>
                <h3 style={{ fontWeight: 'bold', marginBottom: '0.75rem', fontSize: '1rem', color: '#f3f4f6' }}>Status</h3>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                  <span style={{ width: '0.75rem', height: '0.75rem', borderRadius: '50%', background: getStatusColor(details.status) }}></span>
                  <span>{details.status}</span>
                </div>
                <div style={{ marginTop: '0.5rem', color: '#94a3b8', fontSize: '0.875rem' }}>
                  State: {details.state}
                </div>
              </div>

              <div style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155' }}>
                <h3 style={{ fontWeight: 'bold', marginBottom: '0.75rem', color: '#f3f4f6' }}>Resource Usage</h3>
                <div style={{ fontSize: '0.875rem' }}>
                  <div style={{ marginBottom: '0.25rem' }}>
                    <strong>CPU:</strong> {(details.cpu_stats?.cpu_percent || 0).toFixed(2)}%
                  </div>
                  <div style={{ marginBottom: '0.25rem' }}>
                    <strong>Memory:</strong> {(details.memory_stats?.percent || 0).toFixed(2)}%
                    <span style={{ color: '#94a3b8' }}>
                      {' '}({formatBytes(details.memory_stats?.usage || 0)} / {formatBytes(details.memory_stats?.limit || 0)})
                    </span>
                  </div>
                  <div style={{ marginBottom: '0.25rem' }}>
                    <strong>Network RX:</strong> <span style={{ color: '#94a3b8' }}>{formatBytes(details.network_stats?.rx_bytes || 0)}</span>
                  </div>
                  <div>
                    <strong>Network TX:</strong> <span style={{ color: '#94a3b8' }}>{formatBytes(details.network_stats?.tx_bytes || 0)}</span>
                  </div>
                </div>
              </div>
            </div>

            <div style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155', marginBottom: '1.5rem' }}>
              <h3 style={{ fontWeight: 'bold', marginBottom: '0.75rem' }}>Description</h3>
              <p style={{ color: '#e2e8f0' }}>{details.note}</p>
            </div>

            <div style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155' }}>
              <h3 style={{ fontWeight: 'bold', marginBottom: '0.75rem' }}>Details</h3>
              <div style={{ fontSize: '0.875rem', display: 'grid', gap: '0.5rem', color: '#cbd5e1' }}>
                <div><strong>Image:</strong> {details.image}</div>
                <div><strong>Created:</strong> {details.created}</div>
                <div><strong>Started:</strong> {details.started}</div>
              </div>
            </div>
          </div>
        )}

        {selectedTab === 'metrics' && (
          <div>
            <h3 style={{ fontWeight: 'bold', marginBottom: '1rem' }}>Plugin Metrics</h3>
            {details.metrics.length === 0 ? (
              <div style={{ textAlign: 'center', padding: '2rem', color: '#94a3b8' }}>
                No metrics available for this service
              </div>
            ) : (
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(250px, 1fr))', gap: '1rem' }}>
                {details.metrics.map((metric, idx) => (
                  <div key={idx} style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155' }}>
                    <div style={{ fontSize: '0.75rem', color: '#94a3b8', textTransform: 'uppercase', marginBottom: '0.25rem' }}>
                      {metric.category || 'general'}
                    </div>
                    <div style={{ fontWeight: 'bold', marginBottom: '0.25rem' }}>{metric.name}</div>
                    <div style={{ fontSize: '1.25rem', color: '#2563eb' }}>{metric.formatted}</div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {selectedTab === 'config' && (
          <div>
            <h3 style={{ fontWeight: 'bold', marginBottom: '1rem' }}>Environment Variables</h3>
            <div style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155', marginBottom: '1.5rem', maxHeight: '400px', overflowY: 'auto' }}>
              {Object.keys(details.env_vars).length === 0 ? (
                <div style={{ color: '#94a3b8' }}>No environment variables</div>
              ) : (
                <table style={{ width: '100%', fontSize: '0.875rem' }}>
                  <thead>
                    <tr style={{ borderBottom: '1px solid #334155' }}>
                      <th style={{ textAlign: 'left', padding: '0.5rem' }}>Key</th>
                      <th style={{ textAlign: 'left', padding: '0.5rem' }}>Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    {Object.entries(details.env_vars).map(([key, value]) => (
                      <tr key={key} style={{ borderBottom: '1px solid #334155' }}>
                        <td style={{ padding: '0.5rem', fontFamily: 'monospace', color: '#3b82f6' }}>{key}</td>
                        <td style={{ padding: '0.5rem', fontFamily: 'monospace', wordBreak: 'break-all' }}>{value}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>

            {details.command && (
              <div style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155', marginBottom: '1rem' }}>
                <h4 style={{ fontWeight: 'bold', marginBottom: '0.5rem' }}>Command</h4>
                <pre style={{ fontFamily: 'monospace', fontSize: '0.875rem', whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}>
                  {details.command}
                </pre>
              </div>
            )}

            {details.entrypoint && (
              <div style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155' }}>
                <h4 style={{ fontWeight: 'bold', marginBottom: '0.5rem' }}>Entrypoint</h4>
                <pre style={{ fontFamily: 'monospace', fontSize: '0.875rem', whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}>
                  {details.entrypoint}
                </pre>
              </div>
            )}
          </div>
        )}

        {selectedTab === 'storage' && (
          <div>
            <h3 style={{ fontWeight: 'bold', marginBottom: '1rem' }}>Volume Mounts</h3>
            {details.volumes.length === 0 ? (
              <div style={{ textAlign: 'center', padding: '2rem', color: '#94a3b8' }}>
                No volumes mounted
              </div>
            ) : (
              <div style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155' }}>
                <table style={{ width: '100%', fontSize: '0.875rem' }}>
                  <thead>
                    <tr style={{ borderBottom: '2px solid #334155' }}>
                      <th style={{ textAlign: 'left', padding: '0.5rem' }}>Source</th>
                      <th style={{ textAlign: 'center', padding: '0.5rem' }}>→</th>
                      <th style={{ textAlign: 'left', padding: '0.5rem' }}>Destination</th>
                      <th style={{ textAlign: 'center', padding: '0.5rem' }}>Mode</th>
                    </tr>
                  </thead>
                  <tbody>
                    {details.volumes.map((vol, idx) => (
                      <tr key={idx} style={{ borderBottom: '1px solid #334155' }}>
                        <td style={{ padding: '0.5rem', fontFamily: 'monospace', color: '#2563eb' }}>{vol.source}</td>
                        <td style={{ textAlign: 'center', padding: '0.5rem' }}>→</td>
                        <td style={{ padding: '0.5rem', fontFamily: 'monospace', color: '#dc2626' }}>{vol.destination}</td>
                        <td style={{ textAlign: 'center', padding: '0.5rem', color: '#94a3b8' }}>{vol.mode}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        )}

        {selectedTab === 'network' && (
          <div>
            <h3 style={{ fontWeight: 'bold', marginBottom: '1rem' }}>Networks</h3>
            <div style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155', marginBottom: '1.5rem' }}>
              {details.networks.length === 0 ? (
                <div style={{ color: '#94a3b8' }}>No networks</div>
              ) : (
                details.networks.map((net, idx) => (
                  <div key={idx} style={{ marginBottom: '0.75rem', paddingBottom: '0.75rem', borderBottom: idx < details.networks.length - 1 ? '1px solid #334155' : 'none' }}>
                    <div style={{ fontWeight: 'bold', marginBottom: '0.25rem' }}>{net.name}</div>
                    <div style={{ fontSize: '0.875rem', color: '#cbd5e1' }}>
                      <div>IP: {net.ip_address || 'N/A'}</div>
                      <div>Gateway: {net.gateway || 'N/A'}</div>
                    </div>
                  </div>
                ))
              )}
            </div>

            <h3 style={{ fontWeight: 'bold', marginBottom: '1rem' }}>Port Mappings</h3>
            <div style={{ background: '#0f172a', padding: '1rem', borderRadius: '0.375rem', border: '1px solid #334155' }}>
              {details.ports.length === 0 ? (
                <div style={{ color: '#94a3b8' }}>No port mappings</div>
              ) : (
                <table style={{ width: '100%', fontSize: '0.875rem' }}>
                  <thead>
                    <tr style={{ borderBottom: '2px solid #334155' }}>
                      <th style={{ textAlign: 'left', padding: '0.5rem' }}>Host Port</th>
                      <th style={{ textAlign: 'center', padding: '0.5rem' }}>→</th>
                      <th style={{ textAlign: 'left', padding: '0.5rem' }}>Container Port</th>
                      <th style={{ textAlign: 'center', padding: '0.5rem' }}>Protocol</th>
                    </tr>
                  </thead>
                  <tbody>
                    {details.ports.map((port, idx) => (
                      <tr key={idx} style={{ borderBottom: '1px solid #334155' }}>
                        <td style={{ padding: '0.5rem', fontFamily: 'monospace', color: '#2563eb' }}>
                          {port.host_port || 'N/A'}
                        </td>
                        <td style={{ textAlign: 'center', padding: '0.5rem' }}>→</td>
                        <td style={{ padding: '0.5rem', fontFamily: 'monospace', color: '#dc2626' }}>
                          {port.container_port}
                        </td>
                        <td style={{ textAlign: 'center', padding: '0.5rem', color: '#94a3b8' }}>
                          {port.protocol}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Log Viewer Modal */}
      {showLogs && (
        <LogViewer
          serviceName={serviceName}
          onClose={() => setShowLogs(false)}
        />
      )}
    </div>
  )
}
