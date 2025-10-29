import { useState } from 'react'
import UserManagement from './UserManagement'
import SecuritySettings from './SecuritySettings'
import AuditLogs from './AuditLogs'
import './AdminPanel.css'

export default function AdminPanel({ user }) {
  const [activeTab, setActiveTab] = useState('users')

  // Check if user is admin
  const isAdmin = user?.roles?.includes('admin')

  if (!isAdmin) {
    return (
      <div className="admin-panel">
        <div className="access-denied">
          <h2>🔒 Access Denied</h2>
          <p>You need administrator privileges to access this page.</p>
          <p>Current roles: {user?.roles?.join(', ') || 'None'}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="admin-panel">
      <div className="admin-header">
        <h1>⚙️ Administration</h1>
        <p>Manage users, security settings, and audit logs</p>
      </div>

      <div className="admin-tabs">
        <button
          className={`admin-tab ${activeTab === 'users' ? 'active' : ''}`}
          onClick={() => setActiveTab('users')}
        >
          👥 Users
        </button>
        <button
          className={`admin-tab ${activeTab === 'security' ? 'active' : ''}`}
          onClick={() => setActiveTab('security')}
        >
          🔒 Security
        </button>
        <button
          className={`admin-tab ${activeTab === 'audit' ? 'active' : ''}`}
          onClick={() => setActiveTab('audit')}
        >
          📋 Audit Logs
        </button>
      </div>

      <div className="admin-content">
        {activeTab === 'users' && <UserManagement />}
        {activeTab === 'security' && <SecuritySettings />}
        {activeTab === 'audit' && <AuditLogs />}
      </div>
    </div>
  )
}
