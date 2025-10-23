import { useState, useEffect } from 'react';
import { api } from '../services/api';

/**
 * Update notification banner
 * Displays when a new version is available
 * Fetches from /api/version endpoint
 * Supports automatic updates via /api/update endpoint
 */
function UpdateBanner() {
  const [versionInfo, setVersionInfo] = useState(null);
  const [dismissed, setDismissed] = useState(false);
  const [showModal, setShowModal] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [updateStatus, setUpdateStatus] = useState(null);

  useEffect(() => {
    checkForUpdates();
    // Check every 6 hours
    const interval = setInterval(checkForUpdates, 6 * 60 * 60 * 1000);
    return () => clearInterval(interval);
  }, []);

  const checkForUpdates = async () => {
    try {
      const response = await fetch('/api/version');
      if (response.ok) {
        const data = await response.json();
        if (data.success && data.data) {
          setVersionInfo(data.data);
          // Reset dismissed state when new version appears
          if (data.data.update_available) {
            const dismissedVersion = localStorage.getItem('dismissedVersion');
            if (dismissedVersion !== data.data.latest_version) {
              setDismissed(false);
            }
          }
        }
      }
    } catch (error) {
      console.error('Failed to check for updates:', error);
    }
  };

  const handleDismiss = () => {
    setDismissed(true);
    if (versionInfo?.latest_version) {
      localStorage.setItem('dismissedVersion', versionInfo.latest_version);
    }
  };

  const handleUpdateNow = async () => {
    setUpdating(true);
    setUpdateStatus({ message: 'Starting update...', step: 'init' });

    try {
      const token = api.getToken();
      const response = await fetch('/api/update', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`
        }
      });

      const data = await response.json();

      if (data.success && data.data) {
        const status = data.data;
        setUpdateStatus(status);

        if (status.success) {
          // Service will restart in 2 seconds
          setTimeout(() => {
            setUpdateStatus({
              message: 'Restarting service... Please refresh this page in a few seconds.',
              step: 'restarting'
            });
            // Auto-reload after 5 seconds
            setTimeout(() => {
              window.location.reload();
            }, 5000);
          }, 2000);
        }
      } else {
        setUpdateStatus({
          message: data.error || 'Update failed',
          step: 'error',
          success: false
        });
        setUpdating(false);
      }
    } catch (error) {
      setUpdateStatus({
        message: `Update failed: ${error.message}`,
        step: 'error',
        success: false
      });
      setUpdating(false);
    }
  };

  if (!versionInfo || !versionInfo.update_available || dismissed) {
    return null;
  }

  return (
    <>
      <div style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        backgroundColor: '#5bc0de',
        color: 'white',
        padding: '12px 20px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        zIndex: 9999,
        boxShadow: '0 2px 8px rgba(0,0,0,0.15)'
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <span style={{ fontSize: '20px' }}>🎉</span>
          <div>
            <strong>Update Available!</strong>
            <span style={{ marginLeft: '10px', opacity: 0.9 }}>
              v{versionInfo.current_version} → v{versionInfo.latest_version}
            </span>
          </div>
        </div>

        <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
          <button
            onClick={() => setShowModal(true)}
            style={{
              backgroundColor: 'white',
              color: '#5bc0de',
              border: 'none',
              padding: '6px 16px',
              borderRadius: '4px',
              fontWeight: '600',
              cursor: 'pointer',
              fontSize: '14px'
            }}
          >
            View Details
          </button>
          <button
            onClick={handleDismiss}
            style={{
              backgroundColor: 'transparent',
              color: 'white',
              border: '1px solid rgba(255,255,255,0.5)',
              padding: '6px 12px',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '14px'
            }}
          >
            Dismiss
          </button>
        </div>
      </div>

      {/* Modal */}
      {showModal && (
        <div style={{
          position: 'fixed',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          backgroundColor: 'rgba(0,0,0,0.5)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          zIndex: 10000,
          padding: '20px'
        }} onClick={() => !updating && setShowModal(false)}>
          <div style={{
            backgroundColor: 'white',
            borderRadius: '8px',
            maxWidth: '600px',
            width: '100%',
            maxHeight: '80vh',
            overflow: 'auto',
            padding: '24px',
            boxShadow: '0 4px 20px rgba(0,0,0,0.3)'
          }} onClick={(e) => e.stopPropagation()}>
            <h2 style={{ marginTop: 0, color: '#1f2937' }}>
              Version {versionInfo.latest_version} Available
            </h2>

            <div style={{ marginBottom: '20px' }}>
              <p style={{ color: '#6b7280', margin: '8px 0' }}>
                <strong>Current version:</strong> {versionInfo.current_version}
              </p>
              <p style={{ color: '#6b7280', margin: '8px 0' }}>
                <strong>Latest version:</strong> {versionInfo.latest_version}
              </p>
              {versionInfo.published_at && (
                <p style={{ color: '#6b7280', margin: '8px 0' }}>
                  <strong>Released:</strong> {new Date(versionInfo.published_at).toLocaleDateString()}
                </p>
              )}
            </div>

            {versionInfo.release_notes && (
              <div style={{ marginBottom: '20px' }}>
                <h3 style={{ fontSize: '16px', marginBottom: '10px', color: '#374151' }}>
                  Release Notes
                </h3>
                <div style={{
                  backgroundColor: '#f9fafb',
                  padding: '12px',
                  borderRadius: '4px',
                  border: '1px solid #e5e7eb',
                  whiteSpace: 'pre-wrap',
                  fontSize: '14px',
                  color: '#4b5563',
                  maxHeight: '300px',
                  overflow: 'auto'
                }}>
                  {versionInfo.release_notes}
                </div>
              </div>
            )}

            {updateStatus && (
              <div style={{
                backgroundColor: updateStatus.success === false ? '#fee2e2' : '#eff6ff',
                border: `1px solid ${updateStatus.success === false ? '#ef4444' : '#3b82f6'}`,
                borderRadius: '4px',
                padding: '16px',
                marginBottom: '20px'
              }}>
                <p style={{
                  margin: 0,
                  color: updateStatus.success === false ? '#991b1b' : '#1e40af',
                  fontSize: '14px'
                }}>
                  {updateStatus.message}
                </p>
              </div>
            )}

            <div style={{
              backgroundColor: '#eff6ff',
              border: '1px solid #3b82f6',
              borderRadius: '4px',
              padding: '16px',
              marginBottom: '20px'
            }}>
              <h4 style={{ margin: '0 0 10px 0', color: '#1e40af', fontSize: '14px' }}>
                💡 Automatic Update
              </h4>
              <p style={{ margin: 0, color: '#1e40af', fontSize: '14px' }}>
                Click "Update Now" to automatically download and install the latest version.
                The service will restart automatically.
              </p>
            </div>

            <div style={{ display: 'flex', gap: '10px', justifyContent: 'flex-end' }}>
              {!updating && versionInfo.release_url && (
                <a
                  href={versionInfo.release_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{
                    backgroundColor: '#6b7280',
                    color: 'white',
                    padding: '10px 20px',
                    borderRadius: '4px',
                    textDecoration: 'none',
                    fontWeight: '600',
                    fontSize: '14px'
                  }}
                >
                  View on GitHub
                </a>
              )}
              <button
                onClick={handleUpdateNow}
                disabled={updating}
                style={{
                  backgroundColor: updating ? '#9ca3af' : '#5bc0de',
                  color: 'white',
                  border: 'none',
                  padding: '10px 20px',
                  borderRadius: '4px',
                  cursor: updating ? 'not-allowed' : 'pointer',
                  fontWeight: '600',
                  fontSize: '14px'
                }}
              >
                {updating ? 'Updating...' : 'Update Now'}
              </button>
              {!updating && (
                <button
                  onClick={() => setShowModal(false)}
                  style={{
                    backgroundColor: '#e5e7eb',
                    color: '#374151',
                    border: 'none',
                    padding: '10px 20px',
                    borderRadius: '4px',
                    cursor: 'pointer',
                    fontWeight: '600',
                    fontSize: '14px'
                  }}
                >
                  Close
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
}

export default UpdateBanner;
