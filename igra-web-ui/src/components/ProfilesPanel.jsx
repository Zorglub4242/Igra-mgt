import { useState, useEffect } from 'react'
import { api } from '../services/api'

export default function ProfilesPanel() {
  const [profiles, setProfiles] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [actionLoading, setActionLoading] = useState({})

  useEffect(() => {
    loadProfiles()
    const interval = setInterval(loadProfiles, 5000)
    return () => clearInterval(interval)
  }, [])

  async function loadProfiles() {
    try {
      const data = await api.getProfiles()
      setProfiles(data)
      setError(null)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  async function handleProfileAction(profileName, action) {
    setActionLoading(prev => ({ ...prev, [profileName]: action }))
    try {
      if (action === 'start') {
        await api.startProfile(profileName)
      } else if (action === 'stop') {
        await api.stopProfile(profileName)
      }
      await loadProfiles()
    } catch (err) {
      alert(`Error: ${err.message}`)
    } finally {
      setActionLoading(prev => ({ ...prev, [profileName]: null }))
    }
  }

  if (loading) {
    return <div className="loading">Loading profiles...</div>
  }

  if (error) {
    return <div className="error">Error: {error}</div>
  }

  return (
    <div className="card">
      <div className="card-header">
        <h2 className="card-title">Docker Compose Profiles</h2>
        <button className="btn" onClick={loadProfiles}>
          🔄 Refresh
        </button>
      </div>

      <table className="table">
        <thead>
          <tr>
            <th>Profile</th>
            <th>Status</th>
            <th>Services</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {profiles.map(profile => (
            <tr key={profile.name}>
              <td>
                <strong>{profile.name}</strong>
              </td>
              <td>
                {profile.is_active ? (
                  <span className="badge badge-success">Active</span>
                ) : (
                  <span className="badge badge-info">Inactive</span>
                )}
              </td>
              <td>
                <div className="service-list">
                  {profile.services.map(service => (
                    <span key={service} className="service-tag">{service}</span>
                  ))}
                </div>
              </td>
              <td>
                <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
                  <button
                    className="btn btn-sm btn-success"
                    onClick={() => handleProfileAction(profile.name, 'start')}
                    disabled={actionLoading[profile.name] || profile.is_active}
                  >
                    {actionLoading[profile.name] === 'start' ? '...' : '▶'}
                  </button>
                  <button
                    className="btn btn-sm btn-danger"
                    onClick={() => handleProfileAction(profile.name, 'stop')}
                    disabled={actionLoading[profile.name] || !profile.is_active}
                  >
                    {actionLoading[profile.name] === 'stop' ? '...' : '⏹'}
                  </button>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
