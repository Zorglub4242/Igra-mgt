import { useState, useEffect } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { api } from '../services/api'
import LogViewer from './LogViewer'

export default function ServicesPanel() {
  const location = useLocation()
  const [services, setServices] = useState([])
  const [systemServices, setSystemServices] = useState([])
  const [profiles, setProfiles] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [actionLoading, setActionLoading] = useState({})
  const [profileLoading, setProfileLoading] = useState({})
  const [projectLoading, setProjectLoading] = useState({})
  const [selectedService, setSelectedService] = useState(null)
  const [showAllContainers, setShowAllContainers] = useState(false)
  const [showSystemServices, setShowSystemServices] = useState(() => {
    try {
      const saved = localStorage.getItem('show_system_services')
      // Only return false if explicitly set to 'false', otherwise default to true
      return saved !== 'false'
    } catch (e) {
      return true // Show by default
    }
  })

  // Filtering state with include/exclude modes
  const [filters, setFilters] = useState(() => {
    // Check URL parameters first
    const params = new URLSearchParams(location.search)
    const profileParam = params.get('profile')
    const categoryParam = params.get('category')

    try {
      const saved = localStorage.getItem('container_filters')
      let initialFilters = {
        profiles: { mode: 'include', values: [] },
        statuses: { mode: 'include', values: [] },
        categories: { mode: 'include', values: [] },
        project: null,
        name: '',
        showSystemServices: true,
        systemServiceName: ''
      }

      if (saved) {
        const parsed = JSON.parse(saved)
        // Migrate old format to new format
        if (Array.isArray(parsed.profiles)) {
          parsed.profiles = { mode: 'include', values: parsed.profiles }
        }
        if (Array.isArray(parsed.statuses)) {
          parsed.statuses = { mode: 'include', values: parsed.statuses }
        }
        // Add categories if missing (for backwards compatibility)
        if (!parsed.categories) {
          parsed.categories = { mode: 'include', values: [] }
        }
        initialFilters = {
          ...parsed,
          showSystemServices: parsed.showSystemServices ?? true,
          systemServiceName: parsed.systemServiceName || ''
        }
      }

      // Override with URL parameters if present
      if (profileParam) {
        initialFilters.profiles = { mode: 'include', values: [profileParam] }
      }
      if (categoryParam) {
        initialFilters.categories = { mode: 'include', values: [categoryParam] }
      }

      return initialFilters
    } catch (e) {
      console.warn('Failed to load filters from localStorage:', e)
      const defaultFilters = {
        profiles: { mode: 'include', values: [] },
        statuses: { mode: 'include', values: [] },
        categories: { mode: 'include', values: [] },
        project: null,
        name: '',
        showSystemServices: true,
        systemServiceName: ''
      }

      // Apply URL parameters to default filters
      if (profileParam) {
        defaultFilters.profiles = { mode: 'include', values: [profileParam] }
      }
      if (categoryParam) {
        defaultFilters.categories = { mode: 'include', values: [categoryParam] }
      }

      return defaultFilters
    }
  })
  const [nameSearchInput, setNameSearchInput] = useState(filters.name || '')
  const [showFilterModal, setShowFilterModal] = useState(false)
  const [profilesDropdownOpen, setProfilesDropdownOpen] = useState(false)
  const [statusesDropdownOpen, setStatusesDropdownOpen] = useState(false)

  // Collapsible profile groups state
  const [collapsedProfiles, setCollapsedProfiles] = useState(() => {
    try {
      const saved = localStorage.getItem('collapsed_profiles')
      return saved ? JSON.parse(saved) : {}
    } catch (e) {
      console.warn('Failed to load collapsed profiles:', e)
      return {}
    }
  })
  const [collapseAnimating, setCollapseAnimating] = useState({})

  // Collapsible project groups state
  const [collapsedProjects, setCollapsedProjects] = useState(() => {
    try {
      const saved = localStorage.getItem('collapsed_projects')
      return saved ? JSON.parse(saved) : {}
    } catch (e) {
      console.warn('Failed to load collapsed projects:', e)
      return {}
    }
  })
  const [projectAnimating, setProjectAnimating] = useState({})

  // Expandable metrics state
  const [expandedMetrics, setExpandedMetrics] = useState(() => {
    try {
      const saved = localStorage.getItem('expanded_metrics')
      return saved ? JSON.parse(saved) : {}
    } catch (e) {
      console.warn('Failed to load expanded metrics:', e)
      return {}
    }
  })
  const [metricsAnimating, setMetricsAnimating] = useState({})
  const [serviceMetrics, setServiceMetrics] = useState({}) // Full metrics data per service
  const [metricsLoading, setMetricsLoading] = useState({}) // Loading state per service

  // Update filters when URL changes
  useEffect(() => {
    const params = new URLSearchParams(location.search)
    const profileParam = params.get('profile')
    const categoryParam = params.get('category')

    setFilters(prev => {
      const updated = { ...prev }

      // Clear all filters when no URL params (showing all services)
      if (!profileParam && !categoryParam) {
        updated.profiles = { mode: 'include', values: [] }
        updated.categories = { mode: 'include', values: [] }
        updated.project = null
      } else {
        // Apply URL param filters
        if (profileParam) {
          updated.profiles = { mode: 'include', values: [profileParam] }
        } else {
          updated.profiles = { mode: 'include', values: [] }
        }

        if (categoryParam) {
          updated.categories = { mode: 'include', values: [categoryParam] }
        } else {
          updated.categories = { mode: 'include', values: [] }
        }
      }

      return updated
    })
  }, [location.search])

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
  }, [showAllContainers, showSystemServices, filters])

  // Save showSystemServices to localStorage whenever it changes
  useEffect(() => {
    localStorage.setItem('show_system_services', String(showSystemServices))
  }, [showSystemServices])

  // Debounced name search
  useEffect(() => {
    const timer = setTimeout(() => {
      setFilters(prev => {
        const newFilters = { ...prev, name: nameSearchInput }
        localStorage.setItem('container_filters', JSON.stringify(newFilters))
        return newFilters
      })
    }, 300)

    return () => clearTimeout(timer)
  }, [nameSearchInput])

  // Save filters to localStorage whenever they change
  useEffect(() => {
    localStorage.setItem('container_filters', JSON.stringify(filters))
  }, [filters])

  // Cleanup localStorage for deleted services in expandedMetrics
  useEffect(() => {
    if (services.length === 0 && systemServices.length === 0) return

    // Include both Docker and system services in valid services set
    const validServices = new Set([
      ...services.map(s => s.name),
      ...systemServices.map(s => s.name)
    ])
    const currentExpanded = { ...expandedMetrics }
    let needsCleanup = false

    Object.keys(currentExpanded).forEach(serviceName => {
      if (!validServices.has(serviceName)) {
        delete currentExpanded[serviceName]
        needsCleanup = true
      }
    })

    if (needsCleanup) {
      setExpandedMetrics(currentExpanded)
      localStorage.setItem('expanded_metrics', JSON.stringify(currentExpanded))
    }
  }, [services, systemServices])

  async function loadData() {
    try {
      const promises = [
        api.getServices({ showAll: true }),
        api.getProfiles()
      ]

      // Add system services fetch if enabled
      if (showSystemServices) {
        promises.push(api.getSystemServices())
      }

      const results = await Promise.all(promises)
      const servicesData = results[0]
      const profilesData = results[1]
      const systemServicesData = showSystemServices ? results[2] : []

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

      // Transform system services to match Docker service format
      const transformedSystemServices = systemServicesData.map(svc => ({
        ...svc,
        service_type: 'systemd',
        // Transform systemd status to Docker-like status
        status: svc.status === 'running' ? 'Up (healthy)' :
                svc.status === 'stopped' ? 'Exited' :
                svc.status,
        // Ensure project_name is set
        project_name: svc.project_name || 'System Services',
        // No image for system services
        image: svc.service_type || 'systemd',
        // Use detected ports for system services
        ports: svc.ports || [],
        // Use system service metrics if available
        cpu_percent: svc.cpu || 0,
        memory_mb: svc.memory ? svc.memory / (1024 * 1024) : 0,
        container_size_mb: null,
        volume_size_mb: null,
        network_rx_mb: svc.network_rx ? svc.network_rx / (1024 * 1024) : null,
        network_tx_mb: svc.network_tx ? svc.network_tx / (1024 * 1024) : null
      }))

      setSystemServices(transformedSystemServices)

      setProfiles(profilesData)
      setError(null)

      // Refresh metrics for services that are currently expanded
      // Check both Docker and system services
      const allRunningServices = [
        ...servicesData.filter(s => s.status.includes('Up')),
        ...transformedSystemServices.filter(s => s.status.includes('Up'))
      ]

      const expandedServiceNames = Object.keys(expandedMetrics).filter(
        name => expandedMetrics[name] && allRunningServices.some(s => s.name === name)
      )

      if (expandedServiceNames.length > 0) {
        // Fetch metrics for all expanded services in parallel
        const metricsPromises = expandedServiceNames.map(async (serviceName) => {
          try {
            // Detect service type and call appropriate API
            const service = allRunningServices.find(s => s.name === serviceName)
            const details = service?.service_type === 'systemd'
              ? await api.getSystemServiceDetails(serviceName)
              : await api.getServiceDetails(serviceName)
            return { serviceName, metrics: details.metrics || [] }
          } catch (error) {
            console.error(`Failed to refresh metrics for ${serviceName}:`, error)
            return { serviceName, metrics: [] }
          }
        })

        Promise.all(metricsPromises).then(results => {
          setServiceMetrics(prev => {
            const updated = { ...prev }
            results.forEach(({ serviceName, metrics }) => {
              updated[serviceName] = metrics
            })
            return updated
          })
        })
      }
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  async function loadServices() {
    try {
      const data = await api.getServices({ showAll: true })
      setServices(data)
      setError(null)
    } catch (err) {
      setError(err.message)
    }
  }

  async function handleServiceAction(serviceName, action, serviceType) {
    const actionText = action === 'start' ? 'start' : action === 'stop' ? 'stop' : 'restart'
    if (!confirm(`Are you sure you want to ${actionText} ${serviceName}?`)) {
      return
    }

    setActionLoading(prev => ({ ...prev, [serviceName]: action }))
    try {
      // Use different API calls for system services vs Docker containers
      if (serviceType === 'systemd') {
        if (action === 'start') {
          await api.startSystemService(serviceName)
        } else if (action === 'stop') {
          await api.stopSystemService(serviceName)
        } else if (action === 'restart') {
          await api.restartSystemService(serviceName)
        }
      } else {
        if (action === 'start') {
          await api.startService(serviceName)
        } else if (action === 'stop') {
          await api.stopService(serviceName)
        } else if (action === 'restart') {
          await api.restartService(serviceName)
        }
      }
      await loadData()
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

  async function handleProjectAction(projectName, action) {
    const actionText = action === 'start' ? 'start' : 'stop'
    const projectProfiles = hierarchicalGroups[projectName] ? Object.keys(hierarchicalGroups[projectName]) : []

    if (projectProfiles.length === 0) {
      alert('No profiles found in this project')
      return
    }

    if (!confirm(`Are you sure you want to ${actionText} all ${projectProfiles.length} profile(s) in project "${projectName}"?`)) {
      return
    }

    setProjectLoading(prev => ({ ...prev, [projectName]: action }))
    try {
      // Start/stop all profiles in this project
      for (const profileName of projectProfiles) {
        try {
          if (action === 'start') {
            await api.startProfile(profileName)
          } else if (action === 'stop') {
            await api.stopProfile(profileName)
          }
        } catch (err) {
          console.error(`Error ${action}ing profile ${profileName}:`, err)
          // Continue with other profiles even if one fails
        }
      }
      await loadData()
    } catch (err) {
      alert(`Error: ${err.message}`)
    } finally {
      setProjectLoading(prev => ({ ...prev, [projectName]: null }))
    }
  }

  // Toggle functions with animation protection
  function toggleProfileCollapse(profileName) {
    if (collapseAnimating[profileName]) return

    setCollapseAnimating(prev => ({ ...prev, [profileName]: true }))

    setCollapsedProfiles(prev => {
      const newState = { ...prev, [profileName]: !prev[profileName] }
      localStorage.setItem('collapsed_profiles', JSON.stringify(newState))
      return newState
    })

    setTimeout(() => {
      setCollapseAnimating(prev => ({ ...prev, [profileName]: false }))
    }, 300)
  }

  function toggleProjectCollapse(projectName) {
    if (projectAnimating[projectName]) return

    setProjectAnimating(prev => ({ ...prev, [projectName]: true }))

    setCollapsedProjects(prev => {
      const newState = { ...prev, [projectName]: !prev[projectName] }
      localStorage.setItem('collapsed_projects', JSON.stringify(newState))
      return newState
    })

    setTimeout(() => {
      setProjectAnimating(prev => ({ ...prev, [projectName]: false }))
    }, 300)
  }

  async function toggleMetricsExpand(serviceName, serviceType) {
    if (metricsAnimating[serviceName]) return

    setMetricsAnimating(prev => ({ ...prev, [serviceName]: true }))

    const willBeExpanded = !expandedMetrics[serviceName]

    setExpandedMetrics(prev => {
      const newState = { ...prev, [serviceName]: willBeExpanded }
      localStorage.setItem('expanded_metrics', JSON.stringify(newState))
      return newState
    })

    // Fetch full metrics on first expand if not already loaded
    if (willBeExpanded && serviceMetrics[serviceName] === undefined) {
      setMetricsLoading(prev => ({ ...prev, [serviceName]: true }))
      try {
        // Use different API based on service type
        const details = serviceType === 'systemd'
          ? await api.getSystemServiceDetails(serviceName)
          : await api.getServiceDetails(serviceName)
        // Cache the metrics even if empty (empty array indicates no metrics available)
        setServiceMetrics(prev => ({ ...prev, [serviceName]: details.metrics || [] }))
      } catch (error) {
        console.error(`Failed to fetch metrics for ${serviceName}:`, error)
        // Set empty array on error so we don't keep trying
        setServiceMetrics(prev => ({ ...prev, [serviceName]: [] }))
      } finally {
        setMetricsLoading(prev => ({ ...prev, [serviceName]: false }))
      }
    }

    setTimeout(() => {
      setMetricsAnimating(prev => ({ ...prev, [serviceName]: false }))
    }, 300)
  }

  // Filter helper functions
  function toggleFilter(filterType, value) {
    setFilters(prev => {
      const newFilters = { ...prev }
      if (filterType === 'profiles' || filterType === 'statuses') {
        const current = new Set(prev[filterType])
        if (current.has(value)) {
          current.delete(value)
        } else {
          current.add(value)
        }
        newFilters[filterType] = Array.from(current)
      } else if (filterType === 'project') {
        newFilters.project = value === prev.project ? null : value
      }
      return newFilters
    })
  }

  function clearAllFilters() {
    setFilters({
      profiles: { mode: 'include', values: [] },
      statuses: { mode: 'include', values: [] },
      project: null,
      name: '',
      showSystemServices: true,
      systemServiceName: ''
    })
    setNameSearchInput('')
  }

  function getActiveFilterCount() {
    let count = 0
    if (filters.profiles.values && filters.profiles.values.length > 0) count += filters.profiles.values.length
    if (filters.statuses.values && filters.statuses.values.length > 0) count += filters.statuses.values.length
    if (filters.project) count++
    if (filters.name) count++
    if (!filters.showSystemServices) count++ // Hiding system services is a filter
    if (filters.systemServiceName) count++
    return count
  }

  function countMetrics(service) {
    // If we've fetched full metrics for this service, use that count
    if (serviceMetrics[service.name] && serviceMetrics[service.name].length > 0) {
      return serviceMetrics[service.name].length
    }

    // Otherwise, fall back to primary/secondary metrics from initial load
    let count = 0
    if (service.primary_metric) count++
    if (service.secondary_metric) count++
    return count
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
    if (mb === null || mb === undefined) {
      return '-'
    }
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

  // Merge Docker services and system services
  const allServices = [...services, ...systemServices]

  // Client-side filtering with include/exclude logic
  const filteredServices = allServices.filter(service => {
    // System services have their own filtering logic
    if (service.service_type === 'systemd') {
      // Check if system services should be shown
      if (!filters.showSystemServices) {
        return false
      }
      // Filter by system service name (partial match, case-insensitive)
      if (filters.systemServiceName && !service.name.toLowerCase().includes(filters.systemServiceName.toLowerCase())) {
        return false
      }
      // Filter by category
      if (filters.categories.values && filters.categories.values.length > 0) {
        const serviceCategory = service.category || 'Uncategorized'
        const hasMatch = filters.categories.values.includes(serviceCategory)

        if (filters.categories.mode === 'include') {
          if (!hasMatch) return false
        } else {
          if (hasMatch) return false
        }
      }
      // System services can still be filtered by status
      if (filters.statuses.values && filters.statuses.values.length > 0) {
        let status = 'unhealthy'
        if (service.status.includes('Up') && service.status.includes('healthy')) {
          status = 'healthy'
        } else if (service.status.includes('Up')) {
          status = 'running'
        } else if (service.status.includes('Exited')) {
          status = 'stopped'
        }

        const hasMatch = filters.statuses.values.includes(status)

        if (filters.statuses.mode === 'include') {
          if (!hasMatch) return false
        } else {
          if (hasMatch) return false
        }
      }
      return true
    }

    // Docker services filtering (profiles, project, container name)
    // Filter by profiles with include/exclude mode
    if (filters.profiles.values && filters.profiles.values.length > 0) {
      const serviceProfiles = serviceToProfiles[service.name] || []
      const hasMatch = filters.profiles.values.some(p => serviceProfiles.includes(p))

      if (filters.profiles.mode === 'include') {
        // Include mode: show only if matches
        if (!hasMatch) return false
      } else {
        // Exclude mode: hide if matches
        if (hasMatch) return false
      }
    }

    // Filter by status with include/exclude mode
    if (filters.statuses.values && filters.statuses.values.length > 0) {
      let status = 'unhealthy'
      if (service.status.includes('Up') && service.status.includes('healthy')) {
        status = 'healthy'
      } else if (service.status.includes('Up')) {
        status = 'running'
      } else if (service.status.includes('Exited')) {
        status = 'stopped'
      }

      const hasMatch = filters.statuses.values.includes(status)

      if (filters.statuses.mode === 'include') {
        // Include mode: show only if matches
        if (!hasMatch) return false
      } else {
        // Exclude mode: hide if matches
        if (hasMatch) return false
      }
    }

    // Filter by project
    if (filters.project && service.project_name !== filters.project) {
      return false
    }

    // Filter by name (partial match, case-insensitive)
    if (filters.name && !service.name.toLowerCase().includes(filters.name.toLowerCase())) {
      return false
    }

    return true
  })

  // Group services hierarchically: Project → Profile → Containers
  const primaryProfiles = ['kaspad', 'backend', 'frontend-w1', 'frontend-w2', 'frontend-w3', 'frontend-w4', 'frontend-w5']
  const hierarchicalGroups = {} // { projectName: { profileName: [services] } }

  filteredServices.forEach(service => {
    const projectName = service.project_name || 'Other'

    let profileName = 'default'

    // System services use category as the grouping
    if (service.service_type === 'systemd') {
      profileName = service.category || 'Uncategorized'
    } else {
      // Docker services use profiles
      const serviceProfiles = serviceToProfiles[service.name] || []
      const primaryProfile = serviceProfiles.find(p => primaryProfiles.includes(p))
      if (primaryProfile) {
        profileName = primaryProfile
      } else if (serviceProfiles.length > 0) {
        // Use first profile if no primary profile found
        profileName = serviceProfiles[0]
      }
    }

    // Initialize nested structure if needed
    if (!hierarchicalGroups[projectName]) {
      hierarchicalGroups[projectName] = {}
    }
    if (!hierarchicalGroups[projectName][profileName]) {
      hierarchicalGroups[projectName][profileName] = []
    }

    hierarchicalGroups[projectName][profileName].push(service)
  })

  // Sort projects alphabetically
  const sortedProjects = Object.keys(hierarchicalGroups).sort()

  // For each project, sort profiles (primary profiles first, then alphabetically)
  const projectProfiles = {}
  sortedProjects.forEach(projectName => {
    const profiles = Object.keys(hierarchicalGroups[projectName])
    const primary = profiles.filter(p => primaryProfiles.includes(p)).sort((a, b) => {
      return primaryProfiles.indexOf(a) - primaryProfiles.indexOf(b)
    })
    const other = profiles.filter(p => !primaryProfiles.includes(p)).sort()
    projectProfiles[projectName] = [...primary, ...other]
  })

  // Check if we have any services to display
  const hasAnyServices = sortedProjects.length > 0

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

    // Check if service has metrics to display
    const hasMetricsData = () => {
      // If metrics are currently expanded, always show the section (even while refreshing)
      if (expandedMetrics[service.name]) {
        return true
      }
      // If we have fetched full metrics, use that to determine availability
      if (serviceMetrics[service.name] !== undefined) {
        return serviceMetrics[service.name].length > 0
      }
      // Haven't fetched yet - check if service has a plugin available OR has parsed log metrics
      return service.has_metrics || !!(service.primary_metric || service.secondary_metric)
    }

    // Only show metrics section if running AND has actual metrics
    const hasMetrics = service.status.includes('Up') && hasMetricsData()
    const isMetricsExpanded = expandedMetrics[service.name]

    return (
      <>
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
                      {service.ports.map((port, idx) => {
                        // Handle both Docker format (object) and system service format (string)
                        if (typeof port === 'string') {
                          // System service format: "16110/tcp"
                          const portNum = port.split('/')[0]
                          return (
                            <div key={idx} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.875rem' }}>
                              <span style={{ color: '#94a3b8' }}>
                                {port}
                              </span>
                              <a
                                href={`http://${window.location.hostname}:${portNum}`}
                                target="_blank"
                                rel="noopener noreferrer"
                                style={{ color: '#818cf8', textDecoration: 'none', fontSize: '1rem' }}
                                title={`Open port ${portNum}`}
                              >
                                🔗
                              </a>
                            </div>
                          )
                        } else if (port.host_port) {
                          // Docker format with host port
                          return (
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
                          )
                        } else {
                          // Docker format without host port (internal only)
                          return (
                            <span key={idx} style={{ fontSize: '0.875rem', color: '#64748b' }}>
                              {port.container_port}
                            </span>
                          )
                        }
                      })}
                    </div>
                  ) : (
                    <span style={{ color: '#64748b' }}>-</span>
                  )}
                </td>
                <td>{formatCpu(service.cpu_percent)}</td>
                <td>{formatMemory(service.memory_mb)}</td>
                <td>
                  <div style={{ fontSize: '0.875rem' }}>
                    <div>{formatMemory(
                      service.container_size_mb !== null && service.volume_size_mb !== null
                        ? service.container_size_mb + service.volume_size_mb
                        : null
                    )}</div>
                    {service.volume_size_mb !== null && service.volume_size_mb > 0 && (
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
                      onClick={() => handleServiceAction(service.name, 'start', service.service_type)}
                      disabled={actionLoading[service.name] || service.status.includes('Up')}
                      style={{ background: '#047857', color: '#fff', padding: '0', minWidth: '1.5rem', width: '1.5rem', height: '1.5rem', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '0', fontSize: '0.75rem', border: 'none' }}
                    >
                      {actionLoading[service.name] === 'start' ? '...' : '▶'}
                    </button>
                    <button
                      className="btn btn-sm"
                      onClick={() => handleServiceAction(service.name, 'stop', service.service_type)}
                      disabled={actionLoading[service.name] || !service.status.includes('Up')}
                      style={{ background: '#b91c1c', color: '#fff', padding: '0', minWidth: '1.5rem', width: '1.5rem', height: '1.5rem', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '0', fontSize: '0.75rem', border: 'none' }}
                    >
                      {actionLoading[service.name] === 'stop' ? '...' : '⏹'}
                    </button>
                    <button
                      className="btn btn-sm"
                      onClick={() => handleServiceAction(service.name, 'restart', service.service_type)}
                      disabled={actionLoading[service.name]}
                      style={{ background: '#c2410c', color: '#fff', padding: '0', minWidth: '1.5rem', width: '1.5rem', height: '1.5rem', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '0', fontSize: '0.75rem', border: 'none' }}
                    >
                      {actionLoading[service.name] === 'restart' ? '...' : '🔄'}
                    </button>
                    <button
                      className="btn btn-sm"
                      onClick={() => setSelectedService({ name: service.name, type: service.service_type || 'docker' })}
                      style={{ background: '#4f46e5', color: '#fff', padding: '0', minWidth: '1.5rem', width: '1.5rem', height: '1.5rem', display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '0', fontSize: '0.75rem', border: 'none' }}
                    >
                      📋
                    </button>
                  </div>
                </td>
              </tr>
              {hasMetrics && (
                <tr key={`${service.name}-metrics`}>
                  <td colSpan="9" style={{ padding: 0, background: '#0a0e1a' }}>
                    <div
                      onClick={() => !metricsAnimating[service.name] && toggleMetricsExpand(service.name, service.service_type)}
                      style={{
                        padding: '0.5rem 1rem',
                        background: 'rgba(59, 130, 246, 0.05)',
                        cursor: metricsAnimating[service.name] ? 'wait' : 'pointer',
                        borderLeft: '3px solid #3b82f6',
                        transition: 'all 0.3s'
                      }}
                    >
                      <span style={{ marginRight: '0.5rem', fontSize: '0.9rem' }}>
                        {isMetricsExpanded ? '▼' : '▶'}
                      </span>
                      <span style={{ fontSize: '0.875rem', color: '#94a3b8' }}>
                        Metrics
                        {metricsLoading[service.name] ? ' (Loading...)' :
                          serviceMetrics[service.name] ? ` (${serviceMetrics[service.name].length})` :
                          ` (${countMetrics(service)})`}
                      </span>
                      {isMetricsExpanded && (
                        <div style={{
                          marginTop: '0.75rem',
                          paddingTop: '0.5rem',
                          borderTop: '1px solid #1e293b'
                        }}>
                          {metricsLoading[service.name] ? (
                            <div style={{ textAlign: 'center', padding: '1rem', color: '#94a3b8' }}>
                              Loading metrics...
                            </div>
                          ) : serviceMetrics[service.name] && serviceMetrics[service.name].length > 0 ? (
                            <div style={{
                              display: 'grid',
                              gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
                              gap: '0.5rem'
                            }}>
                              {serviceMetrics[service.name].map((metric, idx) => (
                                <div key={idx} style={{
                                  padding: '0.75rem',
                                  background: '#1e293b',
                                  borderRadius: '0.375rem',
                                  border: '1px solid #334155'
                                }}>
                                  <div style={{ fontSize: '0.75rem', color: '#64748b', textTransform: 'uppercase', marginBottom: '0.25rem' }}>
                                    {metric.category || 'general'}
                                  </div>
                                  <div style={{ fontSize: '0.875rem', color: '#94a3b8', marginBottom: '0.25rem' }}>
                                    {metric.name}
                                  </div>
                                  <div style={{ fontSize: '1.125rem', color: '#3b82f6', fontWeight: '500' }}>
                                    {metric.formatted}
                                  </div>
                                </div>
                              ))}
                            </div>
                          ) : (
                            <div style={{ textAlign: 'center', padding: '1rem', color: '#94a3b8' }}>
                              No metrics available
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  </td>
                </tr>
              )}
      </>
    )
  }

  // Get unique projects and profiles for filter dropdowns
  const uniqueProjects = [...new Set(services.map(s => s.project_name).filter(Boolean))]
  const availableStatuses = ['healthy', 'running', 'stopped', 'unhealthy']
  const activeFilterCount = getActiveFilterCount()

  return (
    <>
      <div className="card">
        <div className="card-header">
          <h2 className="card-title">Services</h2>
          <div style={{ display: 'flex', gap: '0.5rem' }}>
            <button
              className="btn"
              onClick={() => setShowFilterModal(true)}
              style={{
                background: activeFilterCount > 0 ? '#3b82f6' : '#334155',
                color: '#fff'
              }}
            >
              🔍 Filters{activeFilterCount > 0 && ` (${activeFilterCount})`}
            </button>
            <button className="btn" onClick={loadData}>
              🔄 Refresh
            </button>
          </div>
        </div>

        {!hasAnyServices && (
          <div style={{
            padding: '3rem 1rem',
            textAlign: 'center',
            color: '#94a3b8'
          }}>
            <div style={{ fontSize: '3rem', marginBottom: '1rem' }}>🔍</div>
            <div style={{ fontSize: '1.125rem', fontWeight: '500', marginBottom: '0.5rem' }}>
              No services match your filters
            </div>
            <div style={{ fontSize: '0.875rem', marginBottom: '1.5rem' }}>
              Try adjusting your filter criteria to see more results
            </div>
            {activeFilterCount > 0 && (
              <button
                className="btn"
                onClick={clearAllFilters}
                style={{
                  background: '#3b82f6',
                  color: '#fff',
                  padding: '0.5rem 1rem',
                  borderRadius: '0.375rem',
                  border: 'none',
                  cursor: 'pointer',
                  fontSize: '0.875rem'
                }}
              >
                Clear All Filters
              </button>
            )}
          </div>
        )}

        {sortedProjects.map(projectName => {
          const displayName = formatProjectName(projectName)
          const projectProfileList = projectProfiles[projectName]

          // Count total services in this project
          const totalServices = projectProfileList.reduce((sum, profileName) => {
            return sum + (hierarchicalGroups[projectName][profileName]?.length || 0)
          }, 0)

          return (
            <div key={projectName} style={{ marginBottom: '1.5rem' }}>
              {/* Project Header */}
              <div style={{
                background: 'rgba(59, 130, 246, 0.15)',
                padding: '0.875rem 1rem',
                borderLeft: '4px solid #3b82f6',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                marginTop: '1rem',
                borderRadius: '0.25rem'
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                  <span
                    onClick={(e) => {
                      e.stopPropagation()
                      toggleProjectCollapse(projectName)
                    }}
                    style={{
                      cursor: projectAnimating[projectName] ? 'wait' : 'pointer',
                      fontSize: '1.2rem',
                      transition: 'transform 0.3s',
                      userSelect: 'none'
                    }}
                  >
                    {collapsedProjects[projectName] ? '▶' : '▼'}
                  </span>
                  <strong style={{ color: '#60a5fa', fontSize: '1.125rem' }}>
                    {displayName}
                  </strong>
                  <span style={{ color: '#94a3b8', fontSize: '0.875rem' }}>
                    ({totalServices} {totalServices === 1 ? 'service' : 'services'})
                  </span>
                </div>
                <div style={{ display: 'flex', gap: '0.5rem' }}>
                  <button
                    className="btn btn-sm btn-success"
                    onClick={() => handleProjectAction(projectName, 'start')}
                    disabled={projectLoading[projectName]}
                    title={`Start all profiles in ${displayName}`}
                  >
                    {projectLoading[projectName] === 'start' ? '...' : '▶ Start All'}
                  </button>
                  <button
                    className="btn btn-sm btn-danger"
                    onClick={() => handleProjectAction(projectName, 'stop')}
                    disabled={projectLoading[projectName]}
                    title={`Stop all profiles in ${displayName}`}
                  >
                    {projectLoading[projectName] === 'stop' ? '...' : '⏹ Stop All'}
                  </button>
                </div>
              </div>

              {/* Profiles within Project */}
              {!collapsedProjects[projectName] && projectProfileList.map(profileName => {
                const profileServices = hierarchicalGroups[projectName][profileName]
                const profile = profiles.find(p => p.name === profileName)

                return (
                  <div key={`${projectName}-${profileName}`} style={{ marginLeft: '1rem', marginTop: '0.5rem' }}>
                    {/* Profile Header */}
                    <div style={{
                      background: 'rgba(99, 102, 241, 0.1)',
                      padding: '0.75rem 1rem',
                      borderLeft: '4px solid #6366f1',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between'
                    }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <span
                          onClick={(e) => {
                            e.stopPropagation()
                            toggleProfileCollapse(profileName)
                          }}
                          style={{
                            cursor: collapseAnimating[profileName] ? 'wait' : 'pointer',
                            fontSize: '1rem',
                            transition: 'transform 0.3s',
                            userSelect: 'none'
                          }}
                        >
                          {collapsedProfiles[profileName] ? '▶' : '▼'}
                        </span>
                        <strong style={{ color: '#818cf8', fontSize: '0.95rem' }}>
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

                    {/* Services Table */}
                    {!collapsedProfiles[profileName] && (
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
                    )}
                  </div>
                )
              })}
            </div>
          )
        })}
      </div>

      {selectedService && (
        <LogViewer
          serviceName={typeof selectedService === 'string' ? selectedService : selectedService.name}
          serviceType={typeof selectedService === 'string' ? 'docker' : selectedService.type}
          onClose={() => setSelectedService(null)}
        />
      )}

      {/* Filter Modal */}
      {showFilterModal && (
        <div
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: 'rgba(0, 0, 0, 0.75)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 1000
          }}
          onClick={() => setShowFilterModal(false)}
        >
          <div
            style={{
              background: '#1e293b',
              borderRadius: '0.5rem',
              padding: '1.5rem',
              maxWidth: '600px',
              width: '90%',
              maxHeight: '80vh',
              overflow: 'auto',
              position: 'relative'
            }}
            onClick={(e) => e.stopPropagation()}
          >
            {/* Modal Header */}
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
              <h3 style={{ fontSize: '1.25rem', fontWeight: 'bold', margin: 0 }}>
                🔍 Filter Services
              </h3>
              <button
                onClick={() => setShowFilterModal(false)}
                style={{
                  background: 'transparent',
                  border: 'none',
                  color: '#94a3b8',
                  fontSize: '1.5rem',
                  cursor: 'pointer',
                  padding: '0',
                  lineHeight: '1'
                }}
              >
                ×
              </button>
            </div>

            {/* Project Filter */}
            <div style={{ marginBottom: '1.5rem' }}>
              <label style={{ fontSize: '0.875rem', fontWeight: '500', marginBottom: '0.5rem', display: 'block' }}>
                Project
              </label>
              <select
                value={filters.project || ''}
                onChange={(e) => setFilters(prev => ({ ...prev, project: e.target.value || null }))}
                style={{
                  width: '100%',
                  padding: '0.5rem',
                  borderRadius: '0.375rem',
                  background: '#0f172a',
                  border: '1px solid #334155',
                  color: '#e2e8f0'
                }}
              >
                <option value="">All Projects</option>
                {uniqueProjects.map(project => (
                  <option key={project} value={project}>{formatProjectName(project)}</option>
                ))}
              </select>
            </div>

            {/* Profiles Filter - Dropdown Style */}
            <div style={{ marginBottom: '1.5rem', position: 'relative' }}>
              <label style={{ fontSize: '0.875rem', fontWeight: '500', marginBottom: '0.5rem', display: 'block' }}>
                Profiles
              </label>
              <div
                onClick={() => setProfilesDropdownOpen(!profilesDropdownOpen)}
                style={{
                  width: '100%',
                  padding: '0.5rem',
                  borderRadius: '0.375rem',
                  background: '#0f172a',
                  border: '1px solid #334155',
                  color: '#e2e8f0',
                  cursor: 'pointer',
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center'
                }}
              >
                <span style={{ fontSize: '0.875rem' }}>
                  {filters.profiles.values.length > 0
                    ? `${filters.profiles.mode === 'include' ? 'Include' : 'Exclude'} (${filters.profiles.values.length} selected)`
                    : 'Select profiles...'}
                </span>
                <span style={{ transform: profilesDropdownOpen ? 'rotate(180deg)' : '', transition: 'transform 0.2s' }}>▼</span>
              </div>

              {profilesDropdownOpen && (
                <div
                  style={{
                    position: 'absolute',
                    top: '100%',
                    left: 0,
                    right: 0,
                    marginTop: '0.25rem',
                    background: '#0f172a',
                    border: '1px solid #334155',
                    borderRadius: '0.375rem',
                    padding: '0.75rem',
                    zIndex: 10,
                    maxHeight: '300px',
                    overflowY: 'auto'
                  }}
                  onClick={(e) => e.stopPropagation()}
                >
                  <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '0.75rem', paddingBottom: '0.5rem', borderBottom: '1px solid #334155' }}>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '0.25rem', cursor: 'pointer' }}>
                      <input
                        type="radio"
                        checked={filters.profiles.mode === 'include'}
                        onChange={() => setFilters(prev => ({
                          ...prev,
                          profiles: { ...prev.profiles, mode: 'include' }
                        }))}
                      />
                      <span style={{ fontSize: '0.875rem' }}>Include</span>
                    </label>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '0.25rem', cursor: 'pointer' }}>
                      <input
                        type="radio"
                        checked={filters.profiles.mode === 'exclude'}
                        onChange={() => setFilters(prev => ({
                          ...prev,
                          profiles: { ...prev.profiles, mode: 'exclude' }
                        }))}
                      />
                      <span style={{ fontSize: '0.875rem' }}>Exclude</span>
                    </label>
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                    {profiles.map(profile => (
                      <label key={profile.name} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', fontSize: '0.875rem', padding: '0.25rem' }}>
                        <input
                          type="checkbox"
                          checked={filters.profiles.values.includes(profile.name)}
                          onChange={() => {
                            setFilters(prev => {
                              const newValues = prev.profiles.values.includes(profile.name)
                                ? prev.profiles.values.filter(p => p !== profile.name)
                                : [...prev.profiles.values, profile.name]
                              return {
                                ...prev,
                                profiles: { ...prev.profiles, values: newValues }
                              }
                            })
                          }}
                        />
                        <span>{profile.name}</span>
                      </label>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* Status Filter - Dropdown Style */}
            <div style={{ marginBottom: '1.5rem', position: 'relative' }}>
              <label style={{ fontSize: '0.875rem', fontWeight: '500', marginBottom: '0.5rem', display: 'block' }}>
                Status
              </label>
              <div
                onClick={() => setStatusesDropdownOpen(!statusesDropdownOpen)}
                style={{
                  width: '100%',
                  padding: '0.5rem',
                  borderRadius: '0.375rem',
                  background: '#0f172a',
                  border: '1px solid #334155',
                  color: '#e2e8f0',
                  cursor: 'pointer',
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center'
                }}
              >
                <span style={{ fontSize: '0.875rem' }}>
                  {filters.statuses.values.length > 0
                    ? `${filters.statuses.mode === 'include' ? 'Include' : 'Exclude'} (${filters.statuses.values.length} selected)`
                    : 'Select statuses...'}
                </span>
                <span style={{ transform: statusesDropdownOpen ? 'rotate(180deg)' : '', transition: 'transform 0.2s' }}>▼</span>
              </div>

              {statusesDropdownOpen && (
                <div
                  style={{
                    position: 'absolute',
                    top: '100%',
                    left: 0,
                    right: 0,
                    marginTop: '0.25rem',
                    background: '#0f172a',
                    border: '1px solid #334155',
                    borderRadius: '0.375rem',
                    padding: '0.75rem',
                    zIndex: 10,
                    maxHeight: '300px',
                    overflowY: 'auto'
                  }}
                  onClick={(e) => e.stopPropagation()}
                >
                  <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '0.75rem', paddingBottom: '0.5rem', borderBottom: '1px solid #334155' }}>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '0.25rem', cursor: 'pointer' }}>
                      <input
                        type="radio"
                        checked={filters.statuses.mode === 'include'}
                        onChange={() => setFilters(prev => ({
                          ...prev,
                          statuses: { ...prev.statuses, mode: 'include' }
                        }))}
                      />
                      <span style={{ fontSize: '0.875rem' }}>Include</span>
                    </label>
                    <label style={{ display: 'flex', alignItems: 'center', gap: '0.25rem', cursor: 'pointer' }}>
                      <input
                        type="radio"
                        checked={filters.statuses.mode === 'exclude'}
                        onChange={() => setFilters(prev => ({
                          ...prev,
                          statuses: { ...prev.statuses, mode: 'exclude' }
                        }))}
                      />
                      <span style={{ fontSize: '0.875rem' }}>Exclude</span>
                    </label>
                  </div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                    {availableStatuses.map(status => (
                      <label key={status} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', fontSize: '0.875rem', padding: '0.25rem' }}>
                        <input
                          type="checkbox"
                          checked={filters.statuses.values.includes(status)}
                          onChange={() => {
                            setFilters(prev => {
                              const newValues = prev.statuses.values.includes(status)
                                ? prev.statuses.values.filter(s => s !== status)
                                : [...prev.statuses.values, status]
                              return {
                                ...prev,
                                statuses: { ...prev.statuses, values: newValues }
                              }
                            })
                          }}
                        />
                        <span style={{ textTransform: 'capitalize' }}>{status}</span>
                      </label>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* Name Search */}
            <div style={{ marginBottom: '1.5rem' }}>
              <label style={{ fontSize: '0.875rem', fontWeight: '500', marginBottom: '0.5rem', display: 'block' }}>
                Container Name
              </label>
              <input
                type="text"
                placeholder="Search containers..."
                value={nameSearchInput}
                onChange={(e) => setNameSearchInput(e.target.value)}
                style={{
                  width: '100%',
                  padding: '0.5rem',
                  borderRadius: '0.375rem',
                  background: '#0f172a',
                  border: '1px solid #334155',
                  color: '#e2e8f0'
                }}
              />
            </div>

            {/* System Services Section */}
            <div style={{ marginBottom: '1.5rem', paddingTop: '1rem', borderTop: '1px solid #334155' }}>
              <label style={{ fontSize: '0.875rem', fontWeight: '600', marginBottom: '0.75rem', display: 'block', color: '#818cf8' }}>
                System Services
              </label>

              {/* Show System Services Toggle */}
              <div style={{ marginBottom: '1rem' }}>
                <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer' }}>
                  <input
                    type="checkbox"
                    checked={filters.showSystemServices}
                    onChange={(e) => setFilters(prev => ({
                      ...prev,
                      showSystemServices: e.target.checked
                    }))}
                  />
                  <span style={{ fontSize: '0.875rem' }}>Show System Services</span>
                </label>
              </div>

              {/* System Service Name Filter */}
              {filters.showSystemServices && (
                <>
                  <div style={{ marginBottom: '1rem' }}>
                    <label style={{ fontSize: '0.875rem', fontWeight: '500', marginBottom: '0.5rem', display: 'block' }}>
                      System Service Name
                    </label>
                    <input
                      type="text"
                      placeholder="Search system services..."
                      value={filters.systemServiceName}
                      onChange={(e) => setFilters(prev => ({
                        ...prev,
                        systemServiceName: e.target.value
                      }))}
                      style={{
                        width: '100%',
                        padding: '0.5rem',
                        borderRadius: '0.375rem',
                        background: '#0f172a',
                        border: '1px solid #334155',
                        color: '#e2e8f0'
                      }}
                    />
                  </div>

                  {/* Category Filter */}
                  <div>
                    <label style={{ fontSize: '0.875rem', fontWeight: '500', marginBottom: '0.5rem', display: 'block' }}>
                      Categories
                    </label>
                    <div style={{ display: 'flex', gap: '1rem', marginBottom: '0.5rem' }}>
                      <label style={{ display: 'flex', alignItems: 'center', gap: '0.25rem', cursor: 'pointer' }}>
                        <input
                          type="radio"
                          checked={filters.categories.mode === 'include'}
                          onChange={() => setFilters(prev => ({
                            ...prev,
                            categories: { ...prev.categories, mode: 'include' }
                          }))}
                        />
                        <span style={{ fontSize: '0.875rem' }}>Include</span>
                      </label>
                      <label style={{ display: 'flex', alignItems: 'center', gap: '0.25rem', cursor: 'pointer' }}>
                        <input
                          type="radio"
                          checked={filters.categories.mode === 'exclude'}
                          onChange={() => setFilters(prev => ({
                            ...prev,
                            categories: { ...prev.categories, mode: 'exclude' }
                          }))}
                        />
                        <span style={{ fontSize: '0.875rem' }}>Exclude</span>
                      </label>
                    </div>
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                      {Array.from(new Set(systemServices.map(s => s.category || 'Uncategorized'))).sort().map(category => (
                        <label key={category} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', fontSize: '0.875rem', padding: '0.25rem' }}>
                          <input
                            type="checkbox"
                            checked={filters.categories.values.includes(category)}
                            onChange={() => {
                              setFilters(prev => {
                                const newValues = prev.categories.values.includes(category)
                                  ? prev.categories.values.filter(c => c !== category)
                                  : [...prev.categories.values, category]
                                return {
                                  ...prev,
                                  categories: { ...prev.categories, values: newValues }
                                }
                              })
                            }}
                          />
                          <span>{category}</span>
                        </label>
                      ))}
                    </div>
                  </div>
                </>
              )}
            </div>

            {/* Modal Actions */}
            <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
              <button
                onClick={clearAllFilters}
                className="btn"
                style={{ background: '#64748b', color: '#fff' }}
              >
                Clear All
              </button>
              <button
                onClick={() => setShowFilterModal(false)}
                className="btn"
                style={{ background: '#3b82f6', color: '#fff' }}
              >
                Apply
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  )
}
