import { useState, useEffect, useRef } from 'react'
import { api } from '../services/api'
import './LoginPage.css'

export default function LoginPage({ onLoginSuccess }) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)
  const [showDefaultWarning, setShowDefaultWarning] = useState(false)
  const [rememberMe, setRememberMe] = useState(false)
  const [showPasswordChange, setShowPasswordChange] = useState(false)
  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const passwordInputRef = useRef(null)

  // Load remembered username on mount
  useEffect(() => {
    const rememberedUsername = localStorage.getItem('remembered_username')
    const wasRemembered = localStorage.getItem('remember_me') === 'true'

    if (rememberedUsername && wasRemembered) {
      setUsername(rememberedUsername)
      setRememberMe(true)
      // Focus password field when username is pre-filled
      setTimeout(() => {
        passwordInputRef.current?.focus()
      }, 0)
    }
  }, [])

  const handleSubmit = async (e) => {
    e.preventDefault()
    setError('')
    setLoading(true)

    try {
      const result = await api.login(username, password)

      // Check if user needs to change password
      if (result.force_password_change) {
        setShowDefaultWarning(true)
        setShowPasswordChange(true)
        setError('')
        setLoading(false)
        return
      }

      // Handle "Remember Me" functionality
      if (rememberMe) {
        localStorage.setItem('remembered_username', username)
        localStorage.setItem('remember_me', 'true')
      } else {
        localStorage.removeItem('remembered_username')
        localStorage.removeItem('remember_me')
      }

      // Login successful - session is now stored in cookies
      if (onLoginSuccess) {
        onLoginSuccess(result)
      }
    } catch (err) {
      setError(err.message || 'Login failed. Please check your credentials.')
    } finally {
      setLoading(false)
    }
  }

  const handlePasswordChange = async (e) => {
    e.preventDefault()
    setError('')

    // Validate passwords match
    if (newPassword !== confirmPassword) {
      setError('Passwords do not match')
      return
    }

    // Validate password strength
    if (newPassword.length < 8) {
      setError('Password must be at least 8 characters long')
      return
    }

    setLoading(true)

    try {
      // Change password using the API
      await api.changePassword(username, password, newPassword)

      // Login with new password
      const result = await api.login(username, newPassword)

      // Handle "Remember Me" functionality
      if (rememberMe) {
        localStorage.setItem('remembered_username', username)
        localStorage.setItem('remember_me', 'true')
      }

      // Login successful
      if (onLoginSuccess) {
        onLoginSuccess(result)
      }
    } catch (err) {
      setError(err.message || 'Failed to change password. Please try again.')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="login-page">
      <div className="login-card">
        <div className="login-header">
          <h1>⚡ KASPA L2 Management</h1>
          <p>{showPasswordChange ? 'Change Password' : 'Management Console'}</p>
        </div>

        {showPasswordChange ? (
          <form onSubmit={handlePasswordChange} className="login-form">
            {error && (
              <div className="login-error">
                ⚠️ {error}
              </div>
            )}

            <div className="login-info">
              <p><strong>⚠️ Default password detected</strong></p>
              <p>You must change your password before continuing.</p>
            </div>

            <div className="form-group">
              <label htmlFor="new-password">New Password</label>
              <input
                id="new-password"
                type="password"
                className="token-input"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                placeholder="Enter new password (min 8 characters)"
                required
                autoFocus
                disabled={loading}
                minLength={8}
              />
            </div>

            <div className="form-group">
              <label htmlFor="confirm-password">Confirm Password</label>
              <input
                id="confirm-password"
                type="password"
                className="token-input"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                placeholder="Confirm new password"
                required
                disabled={loading}
                minLength={8}
              />
            </div>

            <button
              type="submit"
              className="login-button"
              disabled={loading}
            >
              {loading ? 'Changing Password...' : 'Change Password'}
            </button>
          </form>
        ) : (
          <form onSubmit={handleSubmit} className="login-form">
          {error && (
            <div className="login-error">
              ⚠️ {error}
            </div>
          )}

          <div className="form-group">
            <label htmlFor="username">Username</label>
            <input
              id="username"
              type="text"
              className="token-input"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Enter your username"
              required
              autoFocus
              disabled={loading}
            />
          </div>

          <div className="form-group">
            <label htmlFor="password">Password</label>
            <input
              ref={passwordInputRef}
              id="password"
              type="password"
              className="token-input"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Enter your password"
              required
              disabled={loading}
            />
          </div>

          <div className="form-group checkbox-group">
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={rememberMe}
                onChange={(e) => setRememberMe(e.target.checked)}
                disabled={loading}
              />
              <span>Remember me for 30 days</span>
            </label>
          </div>

          <button
            type="submit"
            className="login-button"
            disabled={loading}
          >
            {loading ? 'Signing in...' : 'Sign In'}
          </button>
        </form>
        )}
      </div>
    </div>
  )
}
