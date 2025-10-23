import { useState, useEffect, useRef } from 'react'
import { Line } from 'react-chartjs-2'
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler
} from 'chart.js'
import { api } from '../services/api'

// Register Chart.js components
ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler
)

export default function StoragePanel() {
  const [storage, setStorage] = useState(null)
  const [history, setHistory] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [pruning, setPruning] = useState(false)
  const [chartDays, setChartDays] = useState(30)

  useEffect(() => {
    loadStorage()
    loadHistory()
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

  async function loadHistory() {
    try {
      const data = await api.getStorageHistory()
      setHistory(data)
    } catch (err) {
      console.error('Failed to load storage history:', err)
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

  function formatBytes(bytes) {
    const gb = bytes / (1024 * 1024 * 1024)
    return gb.toFixed(2)
  }

  function getProgressClass(percent) {
    if (percent > 90) return 'danger'
    if (percent > 75) return 'warning'
    return ''
  }

  function prepareChartData() {
    if (!history || history.length === 0) return null

    // Filter by selected time range
    const cutoff = new Date()
    cutoff.setDate(cutoff.getDate() - chartDays)
    const filtered = history.filter(m => new Date(m.timestamp) >= cutoff)

    return {
      labels: filtered.map(m => new Date(m.timestamp).toLocaleDateString()),
      datasets: [
        {
          label: 'Total Used',
          data: filtered.map(m => m.total_used_bytes / (1024 ** 3)),
          borderColor: '#6366f1',
          backgroundColor: 'rgba(99, 102, 241, 0.1)',
          fill: true,
        },
        {
          label: 'Volumes',
          data: filtered.map(m => m.docker_volumes_bytes / (1024 ** 3)),
          borderColor: '#5bc0de',
          backgroundColor: 'rgba(91, 192, 222, 0.1)',
          fill: true,
        },
        {
          label: 'Images',
          data: filtered.map(m => m.docker_images_bytes / (1024 ** 3)),
          borderColor: '#4a90b5',
          backgroundColor: 'rgba(74, 144, 181, 0.1)',
          fill: true,
        },
      ],
    }
  }

  const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        position: 'top',
        labels: {
          color: '#e2e8f0',
        },
      },
      title: {
        display: true,
        text: 'Storage Usage History',
        color: '#e2e8f0',
      },
    },
    scales: {
      x: {
        ticks: { color: '#94a3b8' },
        grid: { color: 'rgba(148, 163, 184, 0.1)' },
      },
      y: {
        ticks: {
          color: '#94a3b8',
          callback: (value) => value.toFixed(1) + ' GB'
        },
        grid: { color: 'rgba(148, 163, 184, 0.1)' },
        title: {
          display: true,
          text: 'Storage (GB)',
          color: '#e2e8f0',
        },
      },
    },
  }

  if (loading) {
    return <div className="loading">Loading storage info...</div>
  }

  if (error) {
    return <div className="error">Error: {error}</div>
  }

  const chartData = prepareChartData()
  const totalVolumesSize = storage.docker_volumes.reduce((sum, v) => sum + v.size_bytes, 0)

  return (
    <div>
      {/* System Disk */}
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
              {formatBytes(storage.system_disk.used_bytes)} GB / {formatBytes(storage.system_disk.total_bytes)} GB
            </span>
            <span style={{ fontWeight: 'bold', color: storage.system_disk.use_percent > 90 ? '#ef4444' : '#818cf8' }}>
              {storage.system_disk.use_percent.toFixed(1)}%
            </span>
          </div>
          <div className="progress-bar">
            <div
              className={`progress-fill ${getProgressClass(storage.system_disk.use_percent)}`}
              style={{ width: `${storage.system_disk.use_percent}%` }}
            />
          </div>
          <div style={{ marginTop: '0.5rem', fontSize: '0.875rem', color: '#94a3b8' }}>
            {storage.system_disk.filesystem} mounted on {storage.system_disk.mount_point}
          </div>
        </div>

        {storage.system_disk.use_percent > 90 && (
          <div style={{ background: 'rgba(239, 68, 68, 0.1)', border: '1px solid #ef4444', borderRadius: '0.5rem', padding: '1rem', marginTop: '1rem' }}>
            <strong style={{ color: '#ef4444' }}>⚠️ Warning:</strong> Disk usage is critically high!
          </div>
        )}
      </div>

      {/* Docker Summary */}
      <div className="card" style={{ marginBottom: '1.5rem' }}>
        <div className="card-header">
          <h2 className="card-title">Docker Storage Summary</h2>
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
              <th>Count</th>
              <th>Size</th>
              <th>Reclaimable</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Images</td>
              <td>{storage.docker_images.active_count} / {storage.docker_images.total_count}</td>
              <td><strong>{formatBytes(storage.docker_images.total_bytes)} GB</strong></td>
              <td style={{ color: '#10b981' }}>{formatBytes(storage.docker_images.reclaimable_bytes)} GB</td>
            </tr>
            <tr>
              <td>Volumes</td>
              <td>{storage.docker_volumes.length}</td>
              <td><strong>{formatBytes(totalVolumesSize)} GB</strong></td>
              <td>-</td>
            </tr>
            <tr>
              <td>Containers</td>
              <td>{storage.docker_containers.active_count} / {storage.docker_containers.total_count}</td>
              <td><strong>{formatBytes(storage.docker_containers.total_bytes)} GB</strong></td>
              <td style={{ color: '#10b981' }}>{formatBytes(storage.docker_containers.reclaimable_bytes)} GB</td>
            </tr>
            <tr>
              <td>Build Cache</td>
              <td>{storage.docker_build_cache.total_count}</td>
              <td><strong>{formatBytes(storage.docker_build_cache.total_bytes)} GB</strong></td>
              <td style={{ color: '#10b981' }}>{formatBytes(storage.docker_build_cache.reclaimable_bytes)} GB</td>
            </tr>
            <tr style={{ borderTop: '2px solid #334155' }}>
              <td colSpan="2"><strong>Total Reclaimable</strong></td>
              <td colSpan="2"><strong style={{ color: '#10b981' }}>{formatBytes(storage.reclaimable_space)} GB</strong></td>
            </tr>
          </tbody>
        </table>
      </div>

      {/* Growth Rate Prediction */}
      {storage.growth_rate && (
        <div className="card" style={{ marginBottom: '1.5rem' }}>
          <h2 className="card-title">Growth Predictions</h2>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1rem' }}>
            <div>
              <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>Growth Rate</div>
              <div style={{ fontSize: '1.25rem', fontWeight: 'bold' }}>
                {formatBytes(storage.growth_rate.bytes_per_day)} GB/day
              </div>
            </div>
            <div>
              <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>Trend</div>
              <div style={{ fontSize: '1.25rem', fontWeight: 'bold', color: storage.growth_rate.trend === 'Growing' ? '#ef4444' : '#10b981' }}>
                {storage.growth_rate.trend}
              </div>
            </div>
            {storage.growth_rate.days_to_full && (
              <div>
                <div style={{ fontSize: '0.875rem', color: '#94a3b8' }}>Days to 90% Full</div>
                <div style={{ fontSize: '1.25rem', fontWeight: 'bold', color: storage.growth_rate.days_to_full < 30 ? '#ef4444' : '#818cf8' }}>
                  {storage.growth_rate.days_to_full} days
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Historical Chart */}
      {chartData && (
        <div className="card" style={{ marginBottom: '1.5rem' }}>
          <div className="card-header">
            <h2 className="card-title">Storage History</h2>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <button
                className={`btn ${chartDays === 7 ? 'btn-primary' : ''}`}
                onClick={() => setChartDays(7)}
                style={{ padding: '0.25rem 0.75rem' }}
              >
                7d
              </button>
              <button
                className={`btn ${chartDays === 30 ? 'btn-primary' : ''}`}
                onClick={() => setChartDays(30)}
                style={{ padding: '0.25rem 0.75rem' }}
              >
                30d
              </button>
              <button
                className={`btn ${chartDays === 90 ? 'btn-primary' : ''}`}
                onClick={() => setChartDays(90)}
                style={{ padding: '0.25rem 0.75rem' }}
              >
                90d
              </button>
            </div>
          </div>
          <div style={{ height: '300px', padding: '1rem' }}>
            <Line data={chartData} options={chartOptions} />
          </div>
        </div>
      )}

      {/* Volumes List */}
      <div className="card">
        <h2 className="card-title">Docker Volumes</h2>
        <table className="table">
          <thead>
            <tr>
              <th>Volume Name</th>
              <th>Size</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {storage.docker_volumes
              .sort((a, b) => b.size_bytes - a.size_bytes)
              .map((vol, idx) => (
                <tr key={idx}>
                  <td>
                    {vol.critical && <span style={{ color: '#ef4444', marginRight: '0.5rem' }}>⚠️</span>}
                    <span style={{ fontFamily: 'monospace', fontSize: '0.875rem' }}>
                      {vol.name}
                    </span>
                  </td>
                  <td><strong>{formatBytes(vol.size_bytes)} GB</strong></td>
                  <td>
                    <span style={{
                      padding: '0.25rem 0.5rem',
                      borderRadius: '0.25rem',
                      fontSize: '0.75rem',
                      backgroundColor: vol.in_use ? 'rgba(16, 185, 129, 0.2)' : 'rgba(148, 163, 184, 0.2)',
                      color: vol.in_use ? '#10b981' : '#94a3b8'
                    }}>
                      {vol.in_use ? 'In Use' : 'Unused'}
                    </span>
                  </td>
                </tr>
              ))}
          </tbody>
        </table>

        <div style={{ marginTop: '1.5rem', padding: '1rem', background: 'rgba(99, 102, 241, 0.1)', borderRadius: '0.5rem', border: '1px solid #6366f1' }}>
          <p style={{ margin: 0, color: '#818cf8', fontSize: '0.875rem' }}>
            ℹ️ <strong>Tip:</strong> Volumes marked with ⚠️ are critical (e.g., viaduct_data). Never delete them. Use "Cleanup" to remove unused Docker images and build cache.
          </p>
        </div>
      </div>
    </div>
  )
}
