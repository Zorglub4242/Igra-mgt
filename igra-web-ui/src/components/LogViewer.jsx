import { useState, useEffect, useRef } from 'react'
import { api } from '../services/api'

export default function LogViewer({ serviceName, onClose }) {
  const [logs, setLogs] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [filter, setFilter] = useState('')
  const [levelFilter, setLevelFilter] = useState('ALL')
  const [liveView, setLiveView] = useState(true)
  const logViewerBodyRef = useRef(null)

  useEffect(() => {
    loadLogs()
    if (liveView) {
      const interval = setInterval(loadLogs, 5000)
      return () => clearInterval(interval)
    }
  }, [serviceName, liveView])

  // Scroll to bottom when logs are loaded
  useEffect(() => {
    if (logViewerBodyRef.current && !loading) {
      logViewerBodyRef.current.scrollTop = logViewerBodyRef.current.scrollHeight
    }
  }, [logs, loading])

  async function loadLogs() {
    try {
      const data = await api.getServiceLogsParsed(serviceName, { tail: 500 })
      setLogs(data)
      setError(null)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  function getLevelColor(level) {
    if (level.includes('ERROR')) return '#ef4444'
    if (level.includes('WARN')) return '#f59e0b'
    if (level.includes('INFO')) return '#10b981'
    if (level.includes('DEBUG')) return '#818cf8'
    if (level.includes('TRACE')) return '#64748b'
    return '#94a3b8'
  }

  // Group logs: Level → Module → Lines (CLI tree view style)
  const groupedLogs = logs
    .filter(log => {
      // Text filter
      if (filter && !log.message.toLowerCase().includes(filter.toLowerCase())) {
        return false
      }
      // Level filter
      if (levelFilter !== 'ALL' && !log.level.includes(levelFilter)) {
        return false
      }
      return true
    })
    .reduce((acc, log) => {
      if (!acc[log.level]) {
        acc[log.level] = {}
      }
      if (!acc[log.level][log.module]) {
        acc[log.level][log.module] = []
      }
      acc[log.level][log.module].push(log)
      return acc
    }, {})

  // Get last displayed level and module for footer
  let lastLevel = ''
  let lastModule = ''
  let totalLines = 0

  const levelOrder = ['ERROR', 'WARN', 'INFO', 'DEBUG', 'TRACE']
  const sortedLevels = Object.keys(groupedLogs).sort((a, b) => {
    const aIndex = levelOrder.findIndex(l => a.includes(l))
    const bIndex = levelOrder.findIndex(l => b.includes(l))
    return aIndex - bIndex
  })

  if (sortedLevels.length > 0) {
    const lastLevelKey = sortedLevels[sortedLevels.length - 1]
    const modules = groupedLogs[lastLevelKey]
    const moduleKeys = Object.keys(modules).sort()
    if (moduleKeys.length > 0) {
      lastLevel = lastLevelKey
      lastModule = moduleKeys[moduleKeys.length - 1]
    }
  }

  // Count total lines
  sortedLevels.forEach(level => {
    Object.keys(groupedLogs[level]).forEach(module => {
      totalLines += groupedLogs[level][module].length
    })
  })

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content modal-large" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Logs: {serviceName}</h2>
          <button className="btn-close" onClick={onClose}>×</button>
        </div>

        <div className="log-controls">
          <input
            type="text"
            className="log-search"
            placeholder="Search logs..."
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
          <select
            value={levelFilter}
            onChange={(e) => setLevelFilter(e.target.value)}
            style={{
              padding: '0.5rem',
              borderRadius: '0.25rem',
              border: '1px solid #374151',
              background: '#1f2937',
              color: '#e5e7eb',
              fontSize: '0.875rem'
            }}
          >
            <option value="ALL">All Levels</option>
            <option value="ERROR">ERROR</option>
            <option value="WARN">WARN</option>
            <option value="INFO">INFO</option>
            <option value="DEBUG">DEBUG</option>
            <option value="TRACE">TRACE</option>
          </select>
          <button
            className={`btn btn-sm ${liveView ? 'btn-success' : ''}`}
            onClick={() => setLiveView(!liveView)}
            title={liveView ? 'Live view enabled' : 'Live view disabled'}
          >
            {liveView ? '⏸' : '▶'} Live
          </button>
          <button className="btn btn-sm" onClick={loadLogs}>🔄 Refresh</button>
        </div>

        <div className="log-viewer-body" ref={logViewerBodyRef}>
          {loading && <div style={{ textAlign: 'center', color: '#64748b' }}>Loading logs...</div>}
          {error && <div className="error">Error: {error}</div>}

          {!loading && !error && (
            <div style={{ fontFamily: 'monospace', fontSize: '0.875rem', whiteSpace: 'pre-wrap' }}>
              {sortedLevels.map(level => {
                const modules = groupedLogs[level]

                return (
                  <div key={level} style={{ marginBottom: '1.5rem' }}>
                    {Object.keys(modules).sort().map(module => {
                      const lines = modules[module]

                      return (
                        <div key={`${level}_${module}`} style={{ marginBottom: '0.75rem' }}>
                          {/* Level + Module Header (CLI style: [INFO ] module) */}
                          <div
                            style={{
                              color: getLevelColor(level),
                              fontWeight: 'bold',
                              marginBottom: '0.25rem'
                            }}
                          >
                            [{level.padEnd(6)}] {module}
                          </div>

                          {/* Log Lines with tree characters */}
                          {lines.map((log, idx) => {
                            const isLast = idx === lines.length - 1
                            const treeChar = isLast ? '└─' : '├─'
                            const levelColor = getLevelColor(level)

                            return (
                              <div
                                key={idx}
                                style={{
                                  fontSize: '0.8125rem',
                                  paddingLeft: '0.5rem',
                                  wordBreak: 'break-word'
                                }}
                              >
                                <span style={{ color: '#64748b' }}>{treeChar} </span>
                                <span style={{ color: '#64748b' }}>{log.timestamp}</span>
                                <span style={{ color: levelColor }}> {log.message}</span>
                              </div>
                            )
                          })}
                        </div>
                      )
                    })}
                  </div>
                )
              })}
            </div>
          )}
        </div>

        <div className="modal-footer" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span>Showing {totalLines} log lines</span>
          {lastLevel && lastModule && (
            <span style={{ color: getLevelColor(lastLevel), fontFamily: 'monospace', fontSize: '0.875rem' }}>
              [{lastLevel}] {lastModule}
            </span>
          )}
        </div>
      </div>
    </div>
  )
}
