import { useState, useEffect } from 'react'
import { api } from '../services/api'

export default function StoragePanel() {
  const [storage, setStorage] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [pruning, setPruning] = useState(false)

  useEffect(() => {
    loadStorage()
  }, [])

  async function loadStorage() {
    try {
      const data = await api.getStorage()
      setStorage(data)
      setError(null)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  async function handlePrune() {
    if (!confirm('This will remove unused Docker images and build cache. Continue?')) {
      return
    }

    setPruning(true)
    try {
      await api.pruneStorage()
      await loadStorage()
      alert('Storage cleanup completed!')
    } catch (err) {
      alert(`Error: ${err.message}`)
    } finally {
      setPruning(false)
    }
  }

  function getProgressClass(percent) {
    if (percent > 90) return 'danger'
    if (percent > 75) return 'warning'
    return ''
  }

  if (loading) {
    return <div className="loading">Loading storage info...</div>
  }

  if (error) {
    return <div className="error">Error: {error}</div>
  }

  return (
    <div>
      <div className="card" style={{ marginBottom: '1.5rem' }}>
        <div className="card-header">
          <h2 className="card-title">System Disk Usage</h2>
          <button className="btn" onClick={loadStorage}>
            🔄 Refresh
          </button>
        </div>

        <div style={{ marginBottom: '1.5rem' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.5rem' }}>
            <span>
              {storage.system_disk_used_gb.toFixed(2)} GB / {storage.system_disk_total_gb.toFixed(2)} GB
            </span>
            <span style={{ fontWeight: 'bold', color: storage.system_disk_used_percent > 90 ? '#ef4444' : '#818cf8' }}>
              {storage.system_disk_used_percent.toFixed(1)}%
            </span>
          </div>
          <div className="progress-bar">
            <div
              className={`progress-fill ${getProgressClass(storage.system_disk_used_percent)}`}
              style={{ width: `${storage.system_disk_used_percent}%` }}
            />
          </div>
        </div>

        {storage.system_disk_used_percent > 90 && (
          <div style={{ background: 'rgba(239, 68, 68, 0.1)', border: '1px solid #ef4444', borderRadius: '0.5rem', padding: '1rem', marginTop: '1rem' }}>
            <strong style={{ color: '#ef4444' }}>⚠️ Warning:</strong> Disk usage is critically high!
          </div>
        )}
      </div>

      <div className="card">
        <div className="card-header">
          <h2 className="card-title">Docker Storage Breakdown</h2>
          <button
            className="btn btn-warning"
            onClick={handlePrune}
            disabled={pruning}
          >
            {pruning ? '⏳ Cleaning...' : '🧹 Cleanup'}
          </button>
        </div>

        <table className="table">
          <thead>
            <tr>
              <th>Category</th>
              <th>Size</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Docker Images</td>
              <td><strong>{storage.docker_images_gb.toFixed(2)} GB</strong></td>
            </tr>
            <tr>
              <td>Docker Volumes</td>
              <td><strong>{storage.docker_volumes_gb.toFixed(2)} GB</strong></td>
            </tr>
            <tr>
              <td>Docker Containers</td>
              <td><strong>{storage.docker_containers_gb.toFixed(2)} GB</strong></td>
            </tr>
            <tr>
              <td>Build Cache</td>
              <td><strong>{storage.docker_build_cache_gb.toFixed(2)} GB</strong></td>
            </tr>
            <tr style={{ borderTop: '2px solid #334155' }}>
              <td><strong>Reclaimable Space</strong></td>
              <td><strong style={{ color: '#10b981' }}>{storage.reclaimable_gb.toFixed(2)} GB</strong></td>
            </tr>
          </tbody>
        </table>

        <div style={{ marginTop: '1.5rem', padding: '1rem', background: 'rgba(99, 102, 241, 0.1)', borderRadius: '0.5rem', border: '1px solid #6366f1' }}>
          <p style={{ margin: 0, color: '#818cf8', fontSize: '0.875rem' }}>
            ℹ️ <strong>Tip:</strong> Use the "Cleanup" button to remove unused Docker images and build cache. This is safe and won't affect running services.
          </p>
        </div>
      </div>
    </div>
  )
}
