import { useState, useEffect } from 'react'
import { useNavigate, useLocation } from 'react-router-dom'
import { api } from '../services/api'

export default function Sidebar({ user, onLogout, collapsed, onToggleCollapse }) {
  // collapsed and onToggleCollapse are now controlled by parent
  const [expandedItems, setExpandedItems] = useState(() => {
    try {
      const saved = localStorage.getItem('sidebar_expanded_items')
      return saved ? JSON.parse(saved) : { services: false }
    } catch (e) {
      return { services: false }
    }
  })
  const [profiles, setProfiles] = useState([])
  const [categories, setCategories] = useState([])
  const [services, setServices] = useState([])
  const [systemServices, setSystemServices] = useState([])
  const [loading, setLoading] = useState(true)

  const navigate = useNavigate()
  const location = useLocation()

  useEffect(() => {
    loadMenuData()
    // Refresh menu data periodically to catch new services
    const interval = setInterval(loadMenuData, 30000) // Every 30 seconds
    return () => clearInterval(interval)
  }, [])

  useEffect(() => {
    localStorage.setItem('sidebar_expanded_items', JSON.stringify(expandedItems))
  }, [expandedItems])

  async function loadMenuData() {
    try {
      const [profilesData, categoriesData, servicesData, systemServicesData] = await Promise.all([
        api.getProfiles(),
        api.getCategories(),
        api.getServices({ showAll: true }),
        api.getSystemServices()
      ])

      setProfiles(profilesData || [])
      setCategories(categoriesData || [])
      setServices(servicesData || [])
      setSystemServices(systemServicesData || [])
    } catch (err) {
      console.error('Failed to load menu data:', err)
    } finally {
      setLoading(false)
    }
  }

  function toggleExpanded(itemKey) {
    setExpandedItems(prev => ({
      ...prev,
      [itemKey]: !prev[itemKey]
    }))
  }

  function handleNavigate(path) {
    navigate(path)
  }

  function isActive(path) {
    return location.pathname === path
  }

  function isActiveWithQuery(path, queryKey, queryValue) {
    if (location.pathname !== path) return false
    const params = new URLSearchParams(location.search)
    return params.get(queryKey) === queryValue
  }

  // Build Docker project/profile/container hierarchy
  function buildDockerHierarchy() {
    const projectMap = {}

    // Build service name to profile mapping from profiles data
    const serviceToProfiles = {}
    profiles.forEach(profile => {
      if (profile.services) {
        profile.services.forEach(serviceName => {
          if (!serviceToProfiles[serviceName]) {
            serviceToProfiles[serviceName] = []
          }
          serviceToProfiles[serviceName].push(profile.name)
        })
      }
    })

    // Group services by project and profile
    services.filter(s => s.service_type !== 'systemd').forEach(service => {
      const project = service.project_name || 'default'
      const serviceProfiles = serviceToProfiles[service.name] || []

      if (!projectMap[project]) {
        projectMap[project] = {}
      }

      // If service has no profiles, put it in an "other" pseudo-profile for that project
      if (serviceProfiles.length === 0) {
        const pseudoProfile = `${project}-containers`
        if (!projectMap[project][pseudoProfile]) {
          projectMap[project][pseudoProfile] = []
        }
        projectMap[project][pseudoProfile].push(service)
        return
      }

      serviceProfiles.forEach(profile => {
        if (!projectMap[project][profile]) {
          projectMap[project][profile] = []
        }
        projectMap[project][profile].push(service)
      })
    })

    // Convert to menu structure, filtering out empty projects
    const hierarchy = Object.entries(projectMap)
      .filter(([projectName, profilesObj]) => Object.keys(profilesObj).length > 0)
      .map(([projectName, profilesObj]) => {
        const profiles = Object.entries(profilesObj)
          .filter(([profileName, containers]) => containers.length > 0)
          .map(([profileName, containers]) => {
            // Clean up the label for pseudo-profiles
            const displayLabel = profileName.endsWith('-containers')
              ? 'Containers'
              : profileName

            return {
              type: 'profile',
              key: `profile-${profileName}`,
              label: displayLabel,
              icon: profileName.endsWith('-containers') ? '📦' : '🏷️',
              expandable: true,
              containers: containers.map(container => ({
                label: container.name,
                path: `/service/${container.name}`,
                active: location.pathname === `/service/${container.name}`
              }))
            }
          })

        return {
          type: 'project',
          key: `project-${projectName}`,
          label: projectName,
          icon: '📦',
          expandable: true,
          profiles: profiles
        }
      })
      .filter(project => project.profiles.length > 0)

    return hierarchy
  }

  // Build System categories/services hierarchy
  function buildSystemHierarchy() {
    const categoryServiceMap = {}

    // Group system services by category
    systemServices.forEach(service => {
      const category = service.category || 'Uncategorized'
      if (!categoryServiceMap[category]) {
        categoryServiceMap[category] = []
      }
      categoryServiceMap[category].push(service)
    })

    // Get category metadata and only include categories with services
    return categories
      .filter(cat => cat.is_active && categoryServiceMap[cat.id] && categoryServiceMap[cat.id].length > 0)
      .sort((a, b) => (a.order || 999) - (b.order || 999))
      .map(category => ({
        type: 'category',
        key: `category-${category.id}`,
        label: category.name,
        icon: category.icon,
        expandable: true,
        services: categoryServiceMap[category.id].map(service => ({
          label: service.name,
          path: `/service/${service.name}`,
          active: location.pathname === `/service/${service.name}`
        }))
      }))
  }

  const menuItems = [
    {
      key: 'services',
      icon: '🐳',
      label: 'Services',
      path: '/services',
      expandable: true,
      submenu: {
        projects: buildDockerHierarchy(),
        categories: buildSystemHierarchy()
      }
    },
    {
      key: 'transactions',
      icon: '📊',
      label: 'Transactions',
      path: '/transactions'
    },
    {
      key: 'wallets',
      icon: '💼',
      label: 'Wallets',
      path: '/wallets'
    },
    {
      key: 'storage',
      icon: '🗄️',
      label: 'Storage',
      path: '/storage'
    },
    {
      key: 'monitoring',
      icon: '🔍',
      label: 'Monitoring',
      path: '/monitoring'
    },
    {
      key: 'settings',
      icon: '⚙️',
      label: 'Settings',
      path: '/settings'
    }
  ]

  return (
    <div className={`sidebar ${collapsed ? 'collapsed' : ''}`}>
      <nav className="sidebar-menu">
        {menuItems.map(item => (
          <div key={item.key} className="sidebar-menu-item">
            <div className={`sidebar-item ${isActive(item.path) ? 'active' : ''}`}>
              <div
                style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', flex: 1, cursor: 'pointer' }}
                onClick={() => handleNavigate(item.path)}
              >
                <span className="sidebar-item-icon">{item.icon}</span>
                {!collapsed && <span className="sidebar-item-label">{item.label}</span>}
              </div>
              {!collapsed && item.expandable && (
                <span
                  className="sidebar-item-expand"
                  onClick={(e) => {
                    e.stopPropagation()
                    toggleExpanded(item.key)
                  }}
                  style={{ cursor: 'pointer' }}
                >
                  {expandedItems[item.key] ? '▼' : '▶'}
                </span>
              )}
            </div>

            {/* Submenu */}
            {item.expandable && expandedItems[item.key] && !collapsed && item.submenu && (
              <div className="sidebar-submenu">
                {/* Docker Projects -> Profiles -> Containers */}
                {item.submenu.projects && item.submenu.projects.map(project => (
                  <div key={project.key}>
                    <div
                      className="sidebar-project-header"
                      onClick={(e) => {
                        e.stopPropagation()
                        toggleExpanded(project.key)
                      }}
                    >
                      {project.icon && <span className="sidebar-subitem-icon">{project.icon}</span>}
                      <span className="sidebar-section-label">{project.label}</span>
                      <span className="sidebar-item-expand">
                        {expandedItems[project.key] ? '▼' : '▶'}
                      </span>
                    </div>

                    {/* Profiles */}
                    {expandedItems[project.key] && project.profiles && (
                      <div className="sidebar-profile-items">
                        {project.profiles.map(profile => (
                          <div key={profile.key}>
                            <div
                              className="sidebar-profile-header"
                              onClick={(e) => {
                                e.stopPropagation()
                                toggleExpanded(profile.key)
                              }}
                            >
                              {profile.icon && <span className="sidebar-subitem-icon">{profile.icon}</span>}
                              <span className="sidebar-section-label">{profile.label}</span>
                              <span className="sidebar-item-expand">
                                {expandedItems[profile.key] ? '▼' : '▶'}
                              </span>
                            </div>

                            {/* Containers */}
                            {expandedItems[profile.key] && profile.containers && (
                              <div className="sidebar-container-items">
                                {profile.containers.map((container, idx) => (
                                  <div
                                    key={idx}
                                    className={`sidebar-subitem ${container.active ? 'active' : ''}`}
                                    onClick={(e) => {
                                      e.stopPropagation()
                                      handleNavigate(container.path)
                                    }}
                                  >
                                    <span className="sidebar-subitem-label">{container.label}</span>
                                  </div>
                                ))}
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                ))}

                {/* System Categories -> Services */}
                {item.submenu.categories && item.submenu.categories.map(category => (
                  <div key={category.key}>
                    <div
                      className="sidebar-category-header"
                      onClick={(e) => {
                        e.stopPropagation()
                        toggleExpanded(category.key)
                      }}
                    >
                      {category.icon && <span className="sidebar-subitem-icon">{category.icon}</span>}
                      <span className="sidebar-section-label">{category.label}</span>
                      <span className="sidebar-item-expand">
                        {expandedItems[category.key] ? '▼' : '▶'}
                      </span>
                    </div>

                    {/* Services */}
                    {expandedItems[category.key] && category.services && (
                      <div className="sidebar-service-items">
                        {category.services.map((service, idx) => (
                          <div
                            key={idx}
                            className={`sidebar-subitem ${service.active ? 'active' : ''}`}
                            onClick={(e) => {
                              e.stopPropagation()
                              handleNavigate(service.path)
                            }}
                          >
                            <span className="sidebar-subitem-label">{service.label}</span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </nav>

      <div className="sidebar-footer">
        {!collapsed && (
          <div className="sidebar-user">
            <div className="sidebar-user-name" title={user?.username}>
              👤 {user?.username}
            </div>
            {user?.roles && (
              <div className="sidebar-user-role">
                {Array.from(user.roles).join(', ')}
              </div>
            )}
          </div>
        )}
        <button
          className="sidebar-logout"
          onClick={onLogout}
          title="Logout"
        >
          <span className="sidebar-item-icon">🚪</span>
          {!collapsed && <span>Logout</span>}
        </button>
      </div>
    </div>
  )
}
