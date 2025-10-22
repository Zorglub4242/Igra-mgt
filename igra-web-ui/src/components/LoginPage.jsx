import { useState } from 'react'
import './LoginPage.css'

export default function LoginPage({ onLogin }) {
  const [token, setToken] = useState('')
  const [error, setError] = useState(null)
  const [loading, setLoading] = useState(false)

  async function handleSubmit(e) {
    e.preventDefault()
    setError(null)
    setLoading(true)

    try {
      // Test the token by making a simple API call
      const response = await fetch('/api/health', {
        headers: {
          'Authorization': `Bearer ${token}`
        }
      })

      if (response.ok) {
        // Token is valid, save it and notify parent
        localStorage.setItem('igra_token', token)
        onLogin(token)
      } else {
        setError('Invalid access token')
      }
    } catch (err) {
      setError('Connection error: ' + err.message)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="login-page">
      <div className="login-card">
        <div className="login-header">
          <h1>⚡ IGRA Orchestra</h1>
          <p>Management Console</p>
        </div>

        <form onSubmit={handleSubmit} className="login-form">
          <div className="form-group">
            <label htmlFor="token">Access Token</label>
            <input
              id="token"
              type="password"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="Enter your access token"
              required
              autoFocus
              className="token-input"
            />
          </div>

          {error && (
            <div className="login-error">
              {error}
            </div>
          )}

          <button
            type="submit"
            className="login-button"
            disabled={loading || !token}
          >
            {loading ? 'Verifying...' : 'Login'}
          </button>
        </form>

        <div className="login-footer">
          <p>
            <strong>Note:</strong> The access token is configured in the <code>IGRA_WEB_TOKEN</code> environment variable on the server.
          </p>
        </div>
      </div>
    </div>
  )
}
