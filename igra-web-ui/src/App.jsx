import { useState, useEffect } from 'react'
import { BrowserRouter as Router, Routes, Route, Navigate, useNavigate, useLocation } from 'react-router-dom'
import './App.css'
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

function AppContent({ isAuthenticated, handleLogout, nodeInfo }) {
  const [activeTab, setActiveTab] = useState('services')
  const navigate = useNavigate()
  const location = useLocation()

  function handleTabClick(tab) {
    setActiveTab(tab)
    // Navigate to home if not already there
    if (location.pathname !== '/') {
      navigate('/')
    }
  }

  return (
    <div className="app">
      <UpdateBanner />
      <header className="header">
        <div className="header-content">
          <h1>⚡ Node Management</h1>
          <div className="header-subtitle">
            {nodeInfo ? (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem' }}>
                <div>
                  <strong style={{ color: '#818cf8' }}>Node:</strong> {nodeInfo.node_id || 'Unknown'} •
                  <strong style={{ color: '#818cf8', marginLeft: '0.5rem' }}>CPU:</strong> {nodeInfo.cpu_info || 'N/A'} •
                  <strong style={{ color: '#818cf8', marginLeft: '0.5rem' }}>RAM:</strong> {nodeInfo.total_memory ? `${nodeInfo.total_memory.toFixed(1)} GB` : 'N/A'} •
                  <strong style={{ color: '#818cf8', marginLeft: '0.5rem' }}>Disk:</strong> {nodeInfo.disk_free && nodeInfo.disk_total ? `${nodeInfo.disk_free.toFixed(1)}/${nodeInfo.disk_total.toFixed(1)} GB` : 'N/A'}
                </div>
                <div style={{ fontSize: '0.875rem', color: '#9ca3af' }}>
                  {nodeInfo.os_name || 'Unknown OS'} • Network: {nodeInfo.network || 'Unknown'}
                </div>
              </div>
            ) : (
              'Layer 2 Node Operations'
            )}
          </div>
        </div>
        <button className="logout-button" onClick={handleLogout}>
          🚪 Logout
        </button>
      </header>

      <nav className="tabs">
        <button
          className={`tab ${activeTab === 'services' ? 'active' : ''}`}
          onClick={() => handleTabClick('services')}
        >
          🐳 Services
        </button>
        <button
          className={`tab ${activeTab === 'transactions' ? 'active' : ''}`}
          onClick={() => handleTabClick('transactions')}
        >
          📊 Transactions
        </button>
        <button
          className={`tab ${activeTab === 'wallets' ? 'active' : ''}`}
          onClick={() => handleTabClick('wallets')}
        >
          💼 Wallets
        </button>
        <button
          className={`tab ${activeTab === 'storage' ? 'active' : ''}`}
          onClick={() => handleTabClick('storage')}
        >
          🗄️ Storage
        </button>
        <button
          className={`tab ${activeTab === 'monitoring' ? 'active' : ''}`}
          onClick={() => handleTabClick('monitoring')}
        >
          🔍 Monitoring
        </button>
        <button
          className={`tab ${activeTab === 'settings' ? 'active' : ''}`}
          onClick={() => handleTabClick('settings')}
        >
          ⚙️ Settings
        </button>
      </nav>

      <main className="main-content">
        <Routes>
          <Route path="/" element={
            <>
              {activeTab === 'services' && <ServicesPanel />}
              {activeTab === 'transactions' && <TransactionsPanel />}
              {activeTab === 'wallets' && <WalletsPanel />}
              {activeTab === 'storage' && <StoragePanel />}
              {activeTab === 'monitoring' && <MonitoringPanel />}
              {activeTab === 'settings' && <ConfigPanel />}
            </>
          } />
          <Route path="/service/:serviceName" element={<ServiceDetails />} />
        </Routes>
      </main>

      <footer className="footer">
        <span>Powered by igra-cli v0.12.0</span>
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

  useEffect(() => {
    // Check if user has a valid token
    const token = api.getToken()
    if (token) {
      setIsAuthenticated(true)
    }
    setAuthChecking(false)
  }, [])

  useEffect(() => {
    // Load node info once authenticated
    if (isAuthenticated) {
      loadNodeInfo()
    }
  }, [isAuthenticated])

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
    }
  }

  function handleLogin(token) {
    api.setToken(token)
    setIsAuthenticated(true)
  }

  function handleLogout() {
    api.clearToken()
    setIsAuthenticated(false)
  }

  if (authChecking) {
    return <div className="loading">Loading...</div>
  }

  if (!isAuthenticated) {
    return <LoginPage onLogin={handleLogin} />
  }

  return (
    <Router>
      <AppContent
        isAuthenticated={isAuthenticated}
        handleLogout={handleLogout}
        nodeInfo={nodeInfo}
      />
    </Router>
  )
}

export default App
