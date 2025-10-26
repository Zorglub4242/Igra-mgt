import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { api } from '../services/api'
import LogViewer from './LogViewer'

export default function ServicesPanel() {
  const [services, setServices] = useState([])
  const [profiles, setProfiles] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [actionLoading, setActionLoading] = useState({})
  const [profileLoading, setProfileLoading] = useState({})
  const [selectedService, setSelectedService] = useState(null)
  const [showAllContainers, setShowAllContainers] = useState(false)

  useEffect(() => {
    // Load show_all preference from localStorage
    const showAll = localStorage.getItem('show_all_containers')
    setShowAllContainers(showAll === 'true')

    // Listen for changes from ConfigPanel
    const handleShowAllChanged = (event) => {
      setShowAllContainers(event.detail.enabled)
    }
    window.addEventListener('showAllContainersChanged', handleShowAllChanged)

    return () => {
      window.removeEventListener('showAllContainersChanged', handleShowAllChanged)
    }
  }, [])

  useEffect(() => {
    loadData()
    const interval = setInterval(loadData, 5000) // Refresh every 5s
    return () => clearInterval(interval)
  }, [showAllContainers])

  async function loadData() {
    try {
      const [servicesData, profilesData] = await Promise.all([
        api.getServices(showAllContainers),
        api.getProfiles()
      ])

      // Merge new data with existing data to preserve metrics that may not be in new data
      setServices(prevServices => {
        if (prevServices.length === 0) return servicesData

        return servicesData.map(newService => {
          const oldService = prevServices.find(s => s.name === newService.name)
          if (!oldService) return newService

          // If new data is missing metrics but old data has them, preserve old metrics
          return {
            ...newService,
            primary_metric: newService.primary_metric || oldService.primary_metric,
            secondary_metric: newService.secondary_metric || oldService.secondary_metric
          }
        })
      })

      setProfiles(profilesData)
      setError(null)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  async function loadServices() {
    try {
      const data = await api.getServices(showAllContainers)
      setServices(data)
      setError(null)
    } catch (err) {
      setError(err.message)
    }
  }

  async function handleServiceAction(serviceName, action) {
    const actionText = action === 'start' ? 'start' : action === 'stop' ? 'stop' : 'restart'
    if (!confirm(`Are you sure you want to ${actionText} ${serviceName}?`)) {
      return
    }

    setActionLoading(prev => ({ ...prev, [serviceName]: action }))
    try {
      if (action === 'start') {
        await api.startService(serviceName)
      } else if (action === 'stop') {
        await api.stopService(serviceName)
      } else if (action === 'restart') {
        await api.restartService(serviceName)
      }
      await loadServices()
    } catch (err) {
      alert(`Error: ${err.message}`)
    } finally {
      setActionLoading(prev => ({ ...prev, [serviceName]: null }))
    }
  }

  async function handleProfileAction(profileName, action) {
    const actionText = action === 'start' ? 'start' : 'stop'
    if (!confirm(`Are you sure you want to ${actionText} profile "${profileName}"?`)) {
      return
    }

    setProfileLoading(prev => ({ ...prev, [profileName]: action }))
    try {
      if (action === 'start') {
        await api.startProfile(profileName)
      } else if (action === 'stop') {
        await api.stopProfile(profileName)
      }
      await loadData()
    } catch (err) {
      alert(`Error: ${err.message}`)
    } finally {
      setProfileLoading(prev => ({ ...prev, [profileName]: null }))
    }
  }

  function getStatusBadge(status) {
    if (status.includes('Up') && status.includes('healthy')) {
      return <span className="badge badge-success">Healthy</span>
    } else if (status.includes('Up')) {
      return <span className="badge badge-warning">Running</span>
    } else if (status.includes('Exited')) {
      return <span className="badge badge-danger">Stopped</span>
    }
    return <span className="badge badge-info">{status}</span>
  }

  function formatCpu(percent) {
    return `${percent.toFixed(2)}%`
  }

  function formatMemory(mb) {
    if (mb > 1024) {
      return `${(mb / 1024).toFixed(2)} GB`
    }
    return `${mb.toFixed(0)} MB`
  }

  // Create a map of service name to profiles
  const serviceToProfiles = {}
  profiles.forEach(profile => {
    profile.services.forEach(serviceName => {
      if (!serviceToProfiles[serviceName]) {
        serviceToProfiles[serviceName] = []
      }
      serviceToProfiles[serviceName].push(profile.name)
    })
  })

  // Group services by primary profile (use main profiles: kaspad, backend, frontend-w*)
  const primaryProfiles = ['kaspad', 'backend', 'frontend-w1', 'frontend-w2', 'frontend-w3', 'frontend-w4', 'frontend-w5']
  const groupedServices = {}
  const otherProjectServices = {}

  services.forEach(service => {
    const serviceProfiles = serviceToProfiles[service.name] || []
    const primaryProfile = serviceProfiles.find(p => primaryProfiles.includes(p))

    if (primaryProfile) {
      if (!groupedServices[primaryProfile]) {
        groupedServices[primaryProfile] = []
      }
      groupedServices[primaryProfile].push(service)
    } else {
      // Group non-IGRA services by their project_name
      const projectName = service.project_name || 'Other'
      if (!otherProjectServices[projectName]) {
        otherProjectServices[projectName] = []
      }
      otherProjectServices[projectName].push(service)
    }
  })

  // Sort groups by profile order
  const sortedGroups = primaryProfiles.filter(p => groupedServices[p])

  // Sort other projects alphabetically
  const sortedOtherProjects = Object.keys(otherProjectServices).sort()

  if (loading) {
    return <div className="loading">Loading services...</div>
  }

  if (error) {
    return <div className="error">Error: {error}</div>
  }

  // Helper function to format project name for display
  function formatProjectName(projectName) {
    if (!projectName) return 'Unknown'

    // Remove only _default suffix
    let name = projectName.replace(/_default$/, '')

    // Convert to title case and replace dashes with spaces
    return name.split('-')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ')
  }

  const renderServiceRow = (service) => {
    // Determine if service is from a non-IGRA project
    const isIgraProject = service.project_name?.includes('igra-orchestra')

    return (
              <tr key={service.name}>
                <td>
                  <Link
                    to={`/service/${service.name}`}
                    style={{
                      color: '#818cf8',
                      textDecoration: 'none',
                      fontWeight: 'bold'
                    }}
                    onMouseOver={(e) => e.target.style.textDecoration = 'underline'}
                    onMouseOut={(e) => e.target.style.textDecoration = 'none'}
                  >
                    {service.name}
                  </Link>
                </td>
                <td>
                  {getStatusBadge(service.status)}
                  {service.status_text && (
                    <div style={{ marginTop: '0.25rem' }}>
                      <span className="badge badge-info">{service.status_text}</span>
                    </div>
                  )}
                  {service.primary_metric && (
                    <div style={{ fontSize: '0.75rem', color: '#94a3b8', marginTop: '0.25rem' }}>
                      {service.primary_metric}
                      {service.secondary_metric && ` • ${service.secondary_metric}`}
                    </div>
                  )}
                </td>
                <td>
                  <div style={{ fontSize: '0.875rem', color: '#94a3b8', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {service.image}
                  </div>
                </td>
                <td>
                  {service.ports && service.ports.length > 0 ? (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
                      {service.ports.map((port, idx) => (
                        port.host_port ? (
                          <div key={idx} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.875rem' }}>
                            <span style={{ color: '#94a3b8' }}>
                              {port.host_port}:{port.container_port}
                            </span>
                            <a
                              href={`http://${window.location.hostname}:${port.host_port}`}
                              target="_blank"
                              rel="noopener noreferrer"
                              style={{ color: '#818cf8', textDecoration: 'none', fontSize: '1rem' }}
                              title={`Open port ${port.host_port}`}
                            >
                              🔗
                            </a>
                          </div>
                        ) : (
                          <span key={idx} style={{ fontSize: '0.875rem', color: '#64748b' }}>
                            {port.container_port}
                          </span>
                        )
                      ))}
                    </div>
                  ) : (
                    <span style={{ color: '#64748b' }}>-</span>
                  )}
                </td>
                <td>{formatCpu(service.cpu_percent)}</td>
                <td>{formatMemory(service.memory_mb)}</td>
                <td>
                  <div style={{ fontSize: '0.875rem' }}>
                    <div>{formatMemory(service.container_size_mb + service.volume_size_mb)}</div>
                    {service.volume_size_mb > 0 && (
                      <div style={{ fontSize: '0.75rem', color: '#64748b' }}>
                        ({formatMemory(service.volume_size_mb)} vol)
                      </div>
                    )}
                  </div>
                </td>
                <td>
                  <div style={{ fontSize: '0.875rem' }}>
                    ↓ {formatMemory(service.network_rx_mb)} / ↑ {formatMemory(service.network_tx_mb)}
                  </div>
                </td>
                <td>
                  <div style={{ display: 'flex', gap: '0', flexWrap: 'wrap' }}>
                    <button
                      className="btn btn-sm"
                      onClick={() => handleServiceAction(service.name, 'start')}
                      disabled={actionLoading[service.name] || service.status.includes('Up')}
                      style={{ background: '#047857', color: '#fff', padding: '0.15rem', minWidth: '1.5rem', width: '1.5rem', height: '1.5rem', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '0', fontSize: '0.75rem', border: 'none' }}
                    >
                      {actionLoading[service.name] === 'start' ? '...' : '▶'}
                    </button>
                    <button
                      className="btn btn-sm"
                      onClick={() => handleServiceAction(service.name, 'stop')}
                      disabled={actionLoading[service.name] || !service.status.includes('Up')}
                      style={{ background: '#b91c1c', color: '#fff', padding: '0.15rem', minWidth: '1.5rem', width: '1.5rem', height: '1.5rem', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '0', fontSize: '0.75rem', border: 'none' }}
                    >
                      {actionLoading[service.name] === 'stop' ? '...' : '⏹'}
                    </button>
                    <button
                      className="btn btn-sm"
                      onClick={() => handleServiceAction(service.name, 'restart')}
                      disabled={actionLoading[service.name]}
                      style={{ background: '#c2410c', color: '#fff', padding: '0.15rem', minWidth: '1.5rem', width: '1.5rem', height: '1.5rem', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '0', fontSize: '0.75rem', border: 'none' }}
                    >
                      {actionLoading[service.name] === 'restart' ? '...' : '🔄'}
                    </button>
                    <button
                      className="btn btn-sm"
                      onClick={() => setSelectedService(service.name)}
                      style={{ background: '#4f46e5', color: '#fff', padding: '0.15rem', minWidth: '1.5rem', width: '1.5rem', height: '1.5rem', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '0', fontSize: '0.75rem', border: 'none' }}
                    >
                      📋
                    </button>
                  </div>
                </td>
              </tr>
    )
  }

  return (
    <>
      <div className="card">
        <div className="card-header">
          <h2 className="card-title">Services</h2>
          <button className="btn" onClick={loadData}>
            🔄 Refresh
          </button>
        </div>

        {sortedGroups.map(profileName => {
          const profileServices = groupedServices[profileName]
          const profile = profiles.find(p => p.name === profileName)

          return (
            <div key={profileName} style={{ marginBottom: '1rem' }}>
              <div style={{
                background: 'rgba(99, 102, 241, 0.1)',
                padding: '0.75rem 1rem',
                borderLeft: '4px solid #6366f1',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                marginTop: '1rem'
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                  <strong style={{ color: '#818cf8', fontSize: '1rem' }}>
                    {profileName}
                  </strong>
                  {profile && profile.is_active && (
                    <span className="badge badge-success" style={{ fontSize: '0.75rem' }}>Active</span>
                  )}
                </div>
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                  <button
                    className="btn btn-sm btn-success"
                    onClick={() => handleProfileAction(profileName, 'start')}
                    disabled={profileLoading[profileName] || (profile && profile.is_active)}
                    title={`Start ${profileName} profile`}
                  >
                    {profileLoading[profileName] === 'start' ? '...' : '▶ Start'}
                  </button>
                  <button
                    className="btn btn-sm btn-danger"
                    onClick={() => handleProfileAction(profileName, 'stop')}
                    disabled={profileLoading[profileName] || (profile && !profile.is_active)}
                    title={`Stop ${profileName} profile`}
                  >
                    {profileLoading[profileName] === 'stop' ? '...' : '⏹ Stop'}
                  </button>
                </div>
              </div>

              <table className="table" style={{ tableLayout: 'fixed', width: '100%' }}>
                <thead>
                  <tr>
                    <th style={{ width: '15%' }}>Service</th>
                    <th style={{ width: '12%' }}>Status</th>
                    <th style={{ width: '16%' }}>Image</th>
                    <th style={{ width: '10%' }}>Ports</th>
                    <th style={{ width: '6%' }}>CPU</th>
                    <th style={{ width: '8%' }}>Memory</th>
                    <th style={{ width: '8%' }}>Storage</th>
                    <th style={{ width: '12%' }}>Network (RX/TX)</th>
                    <th style={{ width: '13%' }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {profileServices.map(service => renderServiceRow(service))}
                </tbody>
              </table>
            </div>
          )
        })}

        {sortedOtherProjects.map(projectName => {
          const projectServices = otherProjectServices[projectName]
          const displayName = formatProjectName(projectName)

          return (
            <div key={projectName} style={{ marginBottom: '1rem' }}>
              <div style={{
                background: 'rgba(100, 116, 139, 0.1)',
                padding: '0.75rem 1rem',
                borderLeft: '4px solid #64748b',
                marginTop: '1rem'
              }}>
                <strong style={{ color: '#94a3b8', fontSize: '1rem' }}>
                  {displayName}
                </strong>
                <span style={{ marginLeft: '0.5rem', color: '#64748b', fontSize: '0.875rem' }}>
                  ({projectServices.length} {projectServices.length === 1 ? 'service' : 'services'})
                </span>
              </div>

              <table className="table" style={{ tableLayout: 'fixed', width: '100%' }}>
                <thead>
                  <tr>
                    <th style={{ width: '15%' }}>Service</th>
                    <th style={{ width: '12%' }}>Status</th>
                    <th style={{ width: '16%' }}>Image</th>
                    <th style={{ width: '10%' }}>Ports</th>
                    <th style={{ width: '6%' }}>CPU</th>
                    <th style={{ width: '8%' }}>Memory</th>
                    <th style={{ width: '8%' }}>Storage</th>
                    <th style={{ width: '12%' }}>Network (RX/TX)</th>
                    <th style={{ width: '13%' }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {projectServices.map(service => renderServiceRow(service))}
                </tbody>
              </table>
            </div>
          )
        })}
      </div>

      {selectedService && (
        <LogViewer
          serviceName={selectedService}
          onClose={() => setSelectedService(null)}
        />
      )}
    </>
  )
}
