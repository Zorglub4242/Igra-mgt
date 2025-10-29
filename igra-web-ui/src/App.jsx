import { useState, useEffect } from 'react'
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom'
import './App.css'
import Sidebar from './components/Sidebar'
import ServicesPanel from './components/ServicesPanel'
import ServiceDetails from './components/ServiceDetails'
import WalletsPanel from './components/WalletsPanel'
import StoragePanel from './components/StoragePanel'
import TransactionsPanel from './components/TransactionsPanel'
import MonitoringPanel from './components/MonitoringPanel'
import ConfigPanel from './components/ConfigPanel'
import LoginPage from './components/LoginPage'
import UpdateBanner from './components/UpdateBanner'
import { api } from './services/api'

function AppContent({ isAuthenticated, handleLogout, nodeInfo, user }) {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    try {
      const saved = localStorage.getItem('sidebar_collapsed')
      return saved === 'true'
    } catch (e) {
      return false
    }
  })

  const [userMenuOpen, setUserMenuOpen] = useState(false)

  // Save collapsed state to localStorage whenever it changes
  useEffect(() => {
    localStorage.setItem('sidebar_collapsed', sidebarCollapsed.toString())
  }, [sidebarCollapsed])

  // Close user menu when clicking outside
  useEffect(() => {
    function handleClickOutside(event) {
      if (userMenuOpen && !event.target.closest('.header-user-menu')) {
        setUserMenuOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [userMenuOpen])

  return (
    <div className="app">
      <UpdateBanner />

      {/* Compact Header */}
      <header className="header">
        <div className={`header-left ${sidebarCollapsed ? 'collapsed' : ''}`}>
          <h1>⚡ Kaspa L2 Node</h1>
          {/* Hamburger Menu Button - Part of header */}
          <button
            className="hamburger-button"
            onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
            title={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            ☰
          </button>
        </div>
        <div className="header-right">
          {nodeInfo ? (
            <div className="header-info">
              <span className="header-info-item">
                <strong>Node:</strong> {nodeInfo.node_id || 'Unknown'}
              </span>
              <span className="header-info-separator">•</span>
              <span className="header-info-item">
                <strong>CPU:</strong> {nodeInfo.cpu_info || 'N/A'}
              </span>
              <span className="header-info-separator">•</span>
              <span className="header-info-item">
                <strong>RAM:</strong> {nodeInfo.total_memory ? `${nodeInfo.total_memory.toFixed(1)} GB` : 'N/A'}
              </span>
              <span className="header-info-separator">•</span>
              <span className="header-info-item">
                <strong>Disk:</strong> {nodeInfo.disk_free && nodeInfo.disk_total ? `${nodeInfo.disk_free.toFixed(1)}/${nodeInfo.disk_total.toFixed(1)} GB` : 'N/A'}
              </span>
            </div>
          ) : (
            <div className="header-info">Layer 2 Node Operations</div>
          )}

          {/* User Menu */}
          <div className="header-user-menu">
            <button
              className="header-user-button"
              onClick={() => setUserMenuOpen(!userMenuOpen)}
              title={user?.username}
            >
              <span>👤</span>
              <span className="header-user-name">{user?.username}</span>
            </button>

            {userMenuOpen && (
              <div className="header-user-dropdown">
                <div
                  className="header-user-dropdown-item logout"
                  onClick={() => {
                    setUserMenuOpen(false)
                    handleLogout()
                  }}
                >
                  <span>🚪</span>
                  <span>Logout</span>
                </div>
              </div>
            )}
          </div>
        </div>
      </header>

      {/* Sidebar */}
      <Sidebar
        user={user}
        onLogout={handleLogout}
        collapsed={sidebarCollapsed}
        onToggleCollapse={() => setSidebarCollapsed(!sidebarCollapsed)}
      />

      {/* Main Content Layout */}
      <div className={`app-body ${sidebarCollapsed ? 'sidebar-collapsed' : ''}`}>
        <main className="main-content">
          <Routes>
            <Route path="/" element={<Navigate to="/services" replace />} />
            <Route path="/services" element={<ServicesPanel />} />
            <Route path="/service/:serviceName" element={<ServiceDetails />} />
            <Route path="/transactions" element={<TransactionsPanel />} />
            <Route path="/wallets" element={<WalletsPanel />} />
            <Route path="/storage" element={<StoragePanel />} />
            <Route path="/monitoring" element={<MonitoringPanel />} />
            <Route path="/settings" element={<ConfigPanel user={user} />} />
          </Routes>
        </main>
      </div>

      <footer className="footer">
        <span>Powered by igra-cli v0.14.0</span>
        <span>•</span>
        <a href="/api/health" target="_blank" rel="noopener noreferrer">
          API Health
        </a>
      </footer>
    </div>
  )
}

function App() {
  const [isAuthenticated, setIsAuthenticated] = useState(false)
  const [authChecking, setAuthChecking] = useState(true)
  const [nodeInfo, setNodeInfo] = useState(null)
  const [user, setUser] = useState(null)

  useEffect(() => {
    // Check if user has a valid session
    checkSession()
  }, [])

  useEffect(() => {
    // Load node info once authenticated
    if (isAuthenticated) {
      loadNodeInfo()
    }
  }, [isAuthenticated])

  async function checkSession() {
    try {
      const sessionData = await api.getSession()
      if (sessionData && sessionData.username) {
        setIsAuthenticated(true)
        setUser(sessionData)
      }
    } catch (err) {
      // No valid session - will show login page
      console.log('No active session')
    } finally {
      setAuthChecking(false)
    }
  }

  async function loadNodeInfo() {
    try {
      const [config, systemInfo] = await Promise.all([
        api.getConfig(),
        api.getSystemInfo()
      ])
      setNodeInfo({
        node_id: config.NODE_ID,
        network: config.NETWORK,
        cpu_info: systemInfo.cpu_model,
        total_memory: systemInfo.memory_total_gb,
        disk_free: systemInfo.disk_free_gb,
        disk_total: systemInfo.disk_total_gb,
        os_name: systemInfo.os_name
      })
    } catch (err) {
      console.error('Failed to load node info:', err)
      // If we get a 401, the session expired
      if (err.message && err.message.includes('Unauthorized')) {
        setIsAuthenticated(false)
        setUser(null)
      }
    }
  }

  async function handleLoginSuccess(sessionData) {
    setIsAuthenticated(true)
    setUser(sessionData)
  }

  async function handleLogout() {
    try {
      await api.logout()
    } catch (err) {
      console.error('Logout error:', err)
    } finally {
      setIsAuthenticated(false)
      setUser(null)
    }
  }

  if (authChecking) {
    return <div className="loading">Loading...</div>
  }

  if (!isAuthenticated) {
    return <LoginPage onLoginSuccess={handleLoginSuccess} />
  }

  return (
    <Router>
      <AppContent
        isAuthenticated={isAuthenticated}
        handleLogout={handleLogout}
        nodeInfo={nodeInfo}
        user={user}
      />
    </Router>
  )
}

export default App
