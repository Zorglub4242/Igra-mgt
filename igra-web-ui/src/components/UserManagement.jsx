import { useState, useEffect } from 'react'
import { api } from '../services/api'
import './UserManagement.css'

export default function UserManagement() {
  const [users, setUsers] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [showAddModal, setShowAddModal] = useState(false)
  const [showPasswordModal, setShowPasswordModal] = useState(false)
  const [showRolesModal, setShowRolesModal] = useState(false)
  const [selectedUser, setSelectedUser] = useState(null)

  useEffect(() => {
    loadUsers()
  }, [])

  async function loadUsers() {
    try {
      setLoading(true)
      setError('')
      const data = await api.getUsers()
      setUsers(data)
    } catch (err) {
      setError(err.message || 'Failed to load users')
    } finally {
      setLoading(false)
    }
  }

  async function handleAddUser(userData) {
    try {
      await api.addUser(userData.username, userData.password, userData.roles)
      await loadUsers()
      setShowAddModal(false)
      setError('')
    } catch (err) {
      setError(err.message || 'Failed to add user')
    }
  }

  async function handleDeleteUser(username) {
    if (!confirm(`Are you sure you want to delete user "${username}"?`)) {
      return
    }

    try {
      await api.deleteUser(username)
      await loadUsers()
      setError('')
    } catch (err) {
      setError(err.message || 'Failed to delete user')
    }
  }

  async function handleToggleEnabled(username, currentEnabled) {
    try {
      // Note: The API doesn't have a toggle endpoint yet, so we'll use reset-password
      // For now, we'll just show a message
      alert('Enable/disable will be implemented via CLI: igra-cli user set-enabled ' + username + ' ' + !currentEnabled)
    } catch (err) {
      setError(err.message || 'Failed to toggle user status')
    }
  }

  async function handleResetPassword(username, newPassword) {
    try {
      await api.resetPassword(username, newPassword)
      setShowPasswordModal(false)
      setSelectedUser(null)
      setError('')
      alert('Password reset successfully')
    } catch (err) {
      setError(err.message || 'Failed to reset password')
    }
  }

  async function handleUpdateRoles(username, newRoles) {
    try {
      await api.updateUserRoles(username, newRoles)
      await loadUsers()
      setShowRolesModal(false)
      setSelectedUser(null)
      setError('')
    } catch (err) {
      setError(err.message || 'Failed to update roles')
    }
  }

  if (loading) {
    return <div className="loading">Loading users...</div>
  }

  return (
    <div className="user-management">
      <div className="um-header">
        <h2>User Management</h2>
        <button className="btn-primary" onClick={() => setShowAddModal(true)}>
          ➕ Add User
        </button>
      </div>

      {error && (
        <div className="error-message">
          ⚠️ {error}
        </div>
      )}

      <div className="users-table">
        <table>
          <thead>
            <tr>
              <th>Username</th>
              <th>Roles</th>
              <th>Status</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {users.map(user => (
              <tr key={user.username}>
                <td>
                  <span className="username">{user.username}</span>
                </td>
                <td>
                  <div className="roles">
                    {user.roles.map(role => (
                      <span key={role} className={`role-badge role-${role.toLowerCase()}`}>
                        {role}
                      </span>
                    ))}
                  </div>
                </td>
                <td>
                  <span className={`status-badge ${user.enabled ? 'enabled' : 'disabled'}`}>
                    {user.enabled ? '✓ Enabled' : '✗ Disabled'}
                  </span>
                </td>
                <td>
                  <div className="actions">
                    <button
                      className="btn-small btn-secondary"
                      onClick={() => {
                        setSelectedUser(user)
                        setShowRolesModal(true)
                      }}
                      title="Edit Roles"
                    >
                      👤 Roles
                    </button>
                    <button
                      className="btn-small btn-secondary"
                      onClick={() => {
                        setSelectedUser(user)
                        setShowPasswordModal(true)
                      }}
                      title="Reset Password"
                    >
                      🔑 Password
                    </button>
                    <button
                      className="btn-small btn-danger"
                      onClick={() => handleDeleteUser(user.username)}
                      title="Delete User"
                      disabled={user.username === 'admin' && users.length === 1}
                    >
                      🗑️ Delete
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {showAddModal && (
        <AddUserModal
          onClose={() => setShowAddModal(false)}
          onSave={handleAddUser}
        />
      )}

      {showPasswordModal && selectedUser && (
        <PasswordModal
          user={selectedUser}
          onClose={() => {
            setShowPasswordModal(false)
            setSelectedUser(null)
          }}
          onSave={(password) => handleResetPassword(selectedUser.username, password)}
        />
      )}

      {showRolesModal && selectedUser && (
        <RolesModal
          user={selectedUser}
          onClose={() => {
            setShowRolesModal(false)
            setSelectedUser(null)
          }}
          onSave={(roles) => handleUpdateRoles(selectedUser.username, roles)}
        />
      )}
    </div>
  )
}

function AddUserModal({ onClose, onSave }) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [roles, setRoles] = useState(['Viewer'])
  const [error, setError] = useState('')

  function handleSubmit(e) {
    e.preventDefault()
    setError('')

    if (password !== confirmPassword) {
      setError('Passwords do not match')
      return
    }

    if (password.length < 8) {
      setError('Password must be at least 8 characters')
      return
    }

    onSave({ username, password, roles })
  }

  function toggleRole(role) {
    if (roles.includes(role)) {
      setRoles(roles.filter(r => r !== role))
    } else {
      setRoles([...roles, role])
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h3>Add New User</h3>
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>

        <form onSubmit={handleSubmit}>
          {error && <div className="error-message">{error}</div>}

          <div className="form-group">
            <label>Username</label>
            <input
              type="text"
              value={username}
              onChange={e => setUsername(e.target.value)}
              required
              autoFocus
              placeholder="Enter username"
            />
          </div>

          <div className="form-group">
            <label>Password</label>
            <input
              type="password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              required
              placeholder="Minimum 8 characters"
            />
          </div>

          <div className="form-group">
            <label>Confirm Password</label>
            <input
              type="password"
              value={confirmPassword}
              onChange={e => setConfirmPassword(e.target.value)}
              required
              placeholder="Re-enter password"
            />
          </div>

          <div className="form-group">
            <label>Roles</label>
            <div className="role-checkboxes">
              {['Admin', 'Operator', 'Viewer'].map(role => (
                <label key={role} className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={roles.includes(role)}
                    onChange={() => toggleRole(role)}
                  />
                  <span>{role}</span>
                </label>
              ))}
            </div>
            <p className="help-text">
              Admin: Full access • Operator: Manage services • Viewer: Read-only
            </p>
          </div>

          <div className="modal-actions">
            <button type="button" className="btn-secondary" onClick={onClose}>
              Cancel
            </button>
            <button type="submit" className="btn-primary">
              Add User
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

function PasswordModal({ user, onClose, onSave }) {
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [error, setError] = useState('')

  function handleSubmit(e) {
    e.preventDefault()
    setError('')

    if (password !== confirmPassword) {
      setError('Passwords do not match')
      return
    }

    if (password.length < 8) {
      setError('Password must be at least 8 characters')
      return
    }

    onSave(password)
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h3>Reset Password for "{user.username}"</h3>
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>

        <form onSubmit={handleSubmit}>
          {error && <div className="error-message">{error}</div>}

          <div className="form-group">
            <label>New Password</label>
            <input
              type="password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              required
              autoFocus
              placeholder="Minimum 8 characters"
            />
          </div>

          <div className="form-group">
            <label>Confirm Password</label>
            <input
              type="password"
              value={confirmPassword}
              onChange={e => setConfirmPassword(e.target.value)}
              required
              placeholder="Re-enter password"
            />
          </div>

          <div className="modal-actions">
            <button type="button" className="btn-secondary" onClick={onClose}>
              Cancel
            </button>
            <button type="submit" className="btn-primary">
              Reset Password
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

function RolesModal({ user, onClose, onSave }) {
  const [selectedRoles, setSelectedRoles] = useState(new Set(user.roles))
  const availableRoles = ['admin', 'operator', 'viewer']

  function toggleRole(role) {
    const newRoles = new Set(selectedRoles)
    if (newRoles.has(role)) {
      newRoles.delete(role)
    } else {
      newRoles.add(role)
    }
    setSelectedRoles(newRoles)
  }

  function handleSubmit(e) {
    e.preventDefault()
    if (selectedRoles.size === 0) {
      alert('User must have at least one role')
      return
    }
    onSave(Array.from(selectedRoles))
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <h3>Edit Roles for {user.username}</h3>
        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label>Roles:</label>
            <div className="role-checkboxes">
              {availableRoles.map(role => (
                <label key={role} className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={selectedRoles.has(role)}
                    onChange={() => toggleRole(role)}
                  />
                  <span className={`role-badge role-${role}`}>
                    {role.charAt(0).toUpperCase() + role.slice(1)}
                  </span>
                </label>
              ))}
            </div>
            <p className="help-text">
              <strong>Admin:</strong> Full access to all features<br />
              <strong>Operator:</strong> Can control services and view data<br />
              <strong>Viewer:</strong> Read-only access
            </p>
          </div>

          <div className="modal-actions">
            <button type="button" className="btn-secondary" onClick={onClose}>
              Cancel
            </button>
            <button type="submit" className="btn-primary">
              Update Roles
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}
