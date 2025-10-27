import { useState, useEffect } from 'react'
import { api } from '../services/api'

/**
 * Log Rotation Configuration Modal
 *
 * Can be used for:
 * - Global configuration (containerName = null)
 * - Per-container configuration (containerName = 'service-name')
 *
 * @param {Object} props
 * @param {boolean} props.isOpen - Whether modal is visible
 * @param {Function} props.onClose - Callback when modal closes
 * @param {string|null} props.containerName - Container name for per-container config, null for global
 * @param {Object} props.globalSettings - Global log rotation settings (for comparison)
 */
export default function LogRotationModal({ isOpen, onClose, containerName, globalSettings }) {
  const [useGlobal, setUseGlobal] = useState(containerName === null)
  const [settings, setSettings] = useState({
    driver: 'json-file',
    max_size: '100m',
    max_file: '3',
  })
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState(null)
  const [successMessage, setSuccessMessage] = useState(null)

  const isGlobalMode = containerName === null

  useEffect(() => {
    if (isOpen && containerName) {
      loadContainerSettings()
    } else if (isOpen && isGlobalMode && globalSettings) {
      setSettings(globalSettings)
    }
  }, [isOpen, containerName, globalSettings])

  async function loadContainerSettings() {
    try {
      setLoading(true)
      setError(null)
      const data = await api.getContainerLogRotation(containerName)
      setSettings(data)

      // Check if using global settings
      if (globalSettings &&
          data.driver === globalSettings.driver &&
          data.max_size === globalSettings.max_size &&
          data.max_file === globalSettings.max_file) {
        setUseGlobal(true)
      } else {
        setUseGlobal(false)
      }
    } catch (err) {
      setError('Failed to load container settings: ' + err.message)
    } finally {
      setLoading(false)
    }
  }

  function handleChange(field, value) {
    setSettings(prev => ({
      ...prev,
      [field]: value
    }))
    setSuccessMessage(null)
    setError(null)
  }

  async function handleSave() {
    try {
      setLoading(true)
      setError(null)
      setSuccessMessage(null)

      if (isGlobalMode) {
        // Update global settings
        await api.updateGlobalLogRotation(settings)
        setSuccessMessage('Global log rotation settings updated! Restart containers to apply changes.')
      } else if (useGlobal) {
        // Remove container override (use global)
        await api.deleteContainerLogRotation(containerName)
        setSuccessMessage(`${containerName} will now use global settings. Restart container to apply changes.`)
      } else {
        // Update per-container settings
        await api.updateContainerLogRotation(containerName, settings)
        setSuccessMessage(`Log rotation updated for ${containerName}! Restart container to apply changes.`)
      }

      // Close modal after 2 seconds
      setTimeout(() => {
        onClose(true) // true indicates successful save
      }, 2000)
    } catch (err) {
      setError('Failed to save settings: ' + err.message)
    } finally {
      setLoading(false)
    }
  }

  function handleCancel() {
    setError(null)
    setSuccessMessage(null)
    onClose(false) // false indicates cancel
  }

  if (!isOpen) return null

  const effectiveSettings = useGlobal && !isGlobalMode ? globalSettings : settings

  return (
    <div style={{
      position: 'fixed',
      top: 0,
      left: 0,
      right: 0,
      bottom: 0,
      backgroundColor: 'rgba(0, 0, 0, 0.7)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      zIndex: 1000,
    }}>
      <div style={{
        backgroundColor: '#1e293b',
        border: '1px solid #334155',
        borderRadius: '0.5rem',
        padding: '1.5rem',
        maxWidth: '500px',
        width: '90%',
        maxHeight: '90vh',
        overflow: 'auto',
      }}>
        <h2 style={{
          margin: '0 0 1rem 0',
          color: '#f1f5f9',
          fontSize: '1.25rem',
        }}>
          {isGlobalMode ? 'Global Log Rotation Settings' : `Log Rotation - ${containerName}`}
        </h2>

        {/* Mode selector (only for per-container) */}
        {!isGlobalMode && (
          <div style={{ marginBottom: '1.5rem' }}>
            <label style={{ display: 'flex', alignItems: 'center', marginBottom: '0.5rem' }}>
              <input
                type="radio"
                checked={useGlobal}
                onChange={() => setUseGlobal(true)}
                style={{ marginRight: '0.5rem' }}
              />
              <span style={{ color: '#cbd5e1' }}>Use Global Defaults</span>
            </label>
            <label style={{ display: 'flex', alignItems: 'center' }}>
              <input
                type="radio"
                checked={!useGlobal}
                onChange={() => setUseGlobal(false)}
                style={{ marginRight: '0.5rem' }}
              />
              <span style={{ color: '#cbd5e1' }}>Custom Settings</span>
            </label>
          </div>
        )}

        {/* Settings form */}
        <div style={{ marginBottom: '1.5rem' }}>
          <div style={{ marginBottom: '1rem' }}>
            <label style={{ display: 'block', marginBottom: '0.25rem', color: '#94a3b8', fontSize: '0.875rem' }}>
              Driver
            </label>
            <select
              value={effectiveSettings.driver}
              onChange={(e) => handleChange('driver', e.target.value)}
              disabled={!isGlobalMode && useGlobal}
              style={{
                width: '100%',
                padding: '0.5rem',
                backgroundColor: '#334155',
                border: '1px solid #475569',
                borderRadius: '0.25rem',
                color: '#f1f5f9',
              }}
            >
              <option value="json-file">json-file</option>
              <option value="syslog">syslog</option>
              <option value="journald">journald</option>
            </select>
          </div>

          <div style={{ marginBottom: '1rem' }}>
            <label style={{ display: 'block', marginBottom: '0.25rem', color: '#94a3b8', fontSize: '0.875rem' }}>
              Max Size (e.g., 10m, 100m, 1g)
            </label>
            <input
              type="text"
              value={effectiveSettings.max_size}
              onChange={(e) => handleChange('max_size', e.target.value)}
              disabled={!isGlobalMode && useGlobal}
              placeholder="100m"
              style={{
                width: '100%',
                padding: '0.5rem',
                backgroundColor: '#334155',
                border: '1px solid #475569',
                borderRadius: '0.25rem',
                color: '#f1f5f9',
              }}
            />
          </div>

          <div style={{ marginBottom: '1rem' }}>
            <label style={{ display: 'block', marginBottom: '0.25rem', color: '#94a3b8', fontSize: '0.875rem' }}>
              Max Files (number of rotated files)
            </label>
            <input
              type="text"
              value={effectiveSettings.max_file}
              onChange={(e) => handleChange('max_file', e.target.value)}
              disabled={!isGlobalMode && useGlobal}
              placeholder="3"
              style={{
                width: '100%',
                padding: '0.5rem',
                backgroundColor: '#334155',
                border: '1px solid #475569',
                borderRadius: '0.25rem',
                color: '#f1f5f9',
              }}
            />
          </div>
        </div>

        {/* Preview of effective settings */}
        {!isGlobalMode && useGlobal && (
          <div style={{
            backgroundColor: '#334155',
            border: '1px solid #475569',
            borderRadius: '0.25rem',
            padding: '0.75rem',
            marginBottom: '1rem',
          }}>
            <div style={{ color: '#94a3b8', fontSize: '0.75rem', marginBottom: '0.5rem' }}>
              Using Global Settings:
            </div>
            <div style={{ color: '#cbd5e1', fontSize: '0.875rem' }}>
              Driver: {globalSettings?.driver || 'json-file'}<br/>
              Max Size: {globalSettings?.max_size || '100m'}<br/>
              Max Files: {globalSettings?.max_file || '3'}
            </div>
          </div>
        )}

        {/* Messages */}
        {error && (
          <div style={{
            backgroundColor: '#7f1d1d',
            border: '1px solid #991b1b',
            borderRadius: '0.25rem',
            padding: '0.75rem',
            marginBottom: '1rem',
            color: '#fca5a5',
            fontSize: '0.875rem',
          }}>
            {error}
          </div>
        )}

        {successMessage && (
          <div style={{
            backgroundColor: '#14532d',
            border: '1px solid #166534',
            borderRadius: '0.25rem',
            padding: '0.75rem',
            marginBottom: '1rem',
            color: '#86efac',
            fontSize: '0.875rem',
          }}>
            {successMessage}
          </div>
        )}

        {/* Warning */}
        <div style={{
          backgroundColor: '#422006',
          border: '1px solid #78350f',
          borderRadius: '0.25rem',
          padding: '0.75rem',
          marginBottom: '1rem',
          color: '#fdba74',
          fontSize: '0.875rem',
        }}>
          ⚠️ Container restart required for changes to take effect
        </div>

        {/* Action buttons */}
        <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
          <button
            onClick={handleCancel}
            disabled={loading}
            className="btn btn-secondary"
            style={{
              padding: '0.5rem 1rem',
              backgroundColor: '#475569',
              border: 'none',
              borderRadius: '0.25rem',
              color: '#f1f5f9',
              cursor: loading ? 'not-allowed' : 'pointer',
            }}
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={loading}
            className="btn btn-primary"
            style={{
              padding: '0.5rem 1rem',
              backgroundColor: '#3b82f6',
              border: 'none',
              borderRadius: '0.25rem',
              color: '#ffffff',
              cursor: loading ? 'not-allowed' : 'pointer',
            }}
          >
            {loading ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  )
}
