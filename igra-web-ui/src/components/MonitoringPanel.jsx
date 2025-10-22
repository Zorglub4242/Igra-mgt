import { useState } from 'react'

export default function MonitoringPanel() {
  const [iframeError, setIframeError] = useState(false)
  const grafanaUrl = 'https://grafana.igralabs.com/public-dashboards/5de66581390e434a823a2206237f793b'

  return (
    <div className="card" style={{ height: 'calc(100vh - 200px)', display: 'flex', flexDirection: 'column' }}>
      <div className="card-header">
        <h2 className="card-title">🔍 Monitoring Dashboard</h2>
      </div>

      {/* Iframe container */}
      <div style={{ flex: 1, overflow: 'hidden', padding: 0 }}>
        {iframeError ? (
          <div style={{
            width: '100%',
            height: '100%',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '2rem',
            gap: '1rem'
          }}>
            <div style={{ fontSize: '3rem' }}>🔒</div>
            <h2 style={{ color: '#e5e7eb', marginBottom: '0.5rem' }}>Unable to Embed Dashboard</h2>
            <p style={{ color: '#9ca3af', textAlign: 'center', maxWidth: '600px' }}>
              The Grafana dashboard cannot be embedded due to security restrictions (X-Frame-Options or CSP).
            </p>
            <a
              href={grafanaUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="btn btn-primary"
              style={{ marginTop: '1rem' }}
            >
              Open Dashboard in New Tab
            </a>
          </div>
        ) : (
          <iframe
            src={grafanaUrl}
            style={{
              width: '100%',
              height: '100%',
              border: 'none',
              display: 'block'
            }}
            title="Igra Labs Monitoring Dashboard"
            onError={() => setIframeError(true)}
          />
        )}
      </div>
    </div>
  )
}
