import { useState, useEffect } from 'react'
import { api } from '../services/api'
import './AuditLogs.css'

export default function AuditLogs() {
  const [logs, setLogs] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [limit, setLimit] = useState(100)
  const [filter, setFilter] = useState('')

  useEffect(() => {
    loadLogs()
  }, [limit])

  async function loadLogs() {
    try {
      setLoading(true)
      setError('')
      const data = await api.getAuditLogs(limit)
      setLogs(data)
    } catch (err) {
      setError(err.message || 'Failed to load audit logs')
    } finally {
      setLoading(false)
    }
  }

  async function handleExport() {
    try {
      const data = await api.exportAuditLogs()
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `audit-logs-${new Date().toISOString().split('T')[0]}.json`
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
    } catch (err) {
      setError(err.message || 'Failed to export logs')
    }
  }

  const filteredLogs = logs.filter(log => {
    if (!filter) return true
    const searchStr = filter.toLowerCase()
    return (
      log.event?.toLowerCase().includes(searchStr) ||
      log.username?.toLowerCase().includes(searchStr) ||
      log.ip?.toLowerCase().includes(searchStr) ||
      log.reason?.toLowerCase().includes(searchStr)
    )
  })

  if (loading) {
    return <div className="loading">Loading audit logs...</div>
  }

  return (
    <div className="audit-logs">
      <div className="audit-header">
        <div>
          <h2>Audit Logs</h2>
          <p className="section-description">
            Security and activity logs for compliance and monitoring
          </p>
        </div>
        <div className="audit-actions">
          <button className="btn-secondary" onClick={handleExport}>
            📥 Export JSON
          </button>
          <button className="btn-primary" onClick={loadLogs}>
            🔄 Refresh
          </button>
        </div>
      </div>

      {error && (
        <div className="error-message">
          ⚠️ {error}
        </div>
      )}

      <div className="audit-controls">
        <div className="control-group">
          <label>Show:</label>
          <select value={limit} onChange={e => setLimit(Number(e.target.value))}>
            <option value={50}>50 entries</option>
            <option value={100}>100 entries</option>
            <option value={250}>250 entries</option>
            <option value={500}>500 entries</option>
            <option value={1000}>1000 entries</option>
          </select>
        </div>

        <div className="control-group">
          <input
            type="text"
            placeholder="Filter logs... (event, user, IP, details)"
            value={filter}
            onChange={e => setFilter(e.target.value)}
            className="filter-input"
          />
        </div>
      </div>

      <div className="logs-stats">
        <span>Showing {filteredLogs.length} of {logs.length} entries</span>
      </div>

      {filteredLogs.length > 0 ? (
        <div className="logs-table">
          <table>
            <thead>
              <tr>
                <th>Timestamp</th>
                <th>Event</th>
                <th>User</th>
                <th>IP Address</th>
                <th>Details</th>
              </tr>
            </thead>
            <tbody>
              {filteredLogs.map((log, idx) => (
                <tr key={idx} className={`event-${log.event?.toLowerCase().replace(/\s+/g, '-')}`}>
                  <td className="timestamp">
                    {formatTimestamp(log.timestamp)}
                  </td>
                  <td>
                    <span className={`event-badge event-${getEventClass(log.event)}`}>
                      {getEventIcon(log.event)} {log.event}
                    </span>
                  </td>
                  <td className="username">{log.username || '-'}</td>
                  <td className="ip-address">
                    <code>{log.ip || '-'}</code>
                  </td>
                  <td className="details">{log.reason || log.resource || '-'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <div className="empty-state">
          <p>
            {filter ? 'No logs match your filter' : 'No audit logs available'}
          </p>
        </div>
      )}
    </div>
  )
}

function formatTimestamp(timestamp) {
  if (!timestamp) return '-'
  // Handle both ISO 8601 strings and Unix timestamps
  const date = typeof timestamp === 'string'
    ? new Date(timestamp)
    : new Date(timestamp * 1000)
  return date.toLocaleString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
}

function getEventClass(eventType) {
  if (!eventType) return 'default'
  const lower = eventType.toLowerCase()
  if (lower.includes('success') || lower.includes('login')) return 'success'
  if (lower.includes('fail') || lower.includes('denied')) return 'error'
  if (lower.includes('create') || lower.includes('add')) return 'create'
  if (lower.includes('delete') || lower.includes('remove')) return 'delete'
  if (lower.includes('update') || lower.includes('modify')) return 'update'
  return 'default'
}

function getEventIcon(eventType) {
  if (!eventType) return '📝'
  const lower = eventType.toLowerCase()
  if (lower.includes('login')) return '🔓'
  if (lower.includes('logout')) return '🔒'
  if (lower.includes('fail') || lower.includes('denied')) return '❌'
  if (lower.includes('success')) return '✓'
  if (lower.includes('create') || lower.includes('add')) return '➕'
  if (lower.includes('delete') || lower.includes('remove')) return '🗑️'
  if (lower.includes('update') || lower.includes('modify')) return '✏️'
  return '📝'
}
