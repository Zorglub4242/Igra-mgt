import { useState, useEffect } from 'react'
import { api } from '../services/api'
import WalletDetailModal from './WalletDetailModal'

export default function WalletsPanel() {
  const [wallets, setWallets] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [selectedWallet, setSelectedWallet] = useState(null)
  const [copiedAddress, setCopiedAddress] = useState(null)

  useEffect(() => {
    loadWallets()
  }, [])

  async function loadWallets() {
    try {
      const data = await api.getWallets()
      setWallets(data)
      setError(null)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  function getStatusBadge(containerRunning) {
    if (containerRunning) {
      return <span className="badge badge-success">Running</span>
    } else {
      return <span className="badge badge-danger">Stopped</span>
    }
  }

  async function copyAddress(address, e) {
    e.stopPropagation() // Prevent row click
    try {
      await navigator.clipboard.writeText(address)
      setCopiedAddress(address)
      setTimeout(() => setCopiedAddress(null), 2000)
    } catch (err) {
      console.error('Failed to copy:', err)
    }
  }

  if (loading) {
    return <div className="loading">Loading wallets...</div>
  }

  if (error) {
    return <div className="error">Error: {error}</div>
  }

  return (
    <>
      <div className="card">
        <div className="card-header">
          <h2 className="card-title">Kaspa Wallets</h2>
          <button className="btn" onClick={loadWallets}>
            🔄 Refresh
          </button>
        </div>

        <table className="table">
          <thead>
            <tr>
              <th>Wallet ID</th>
              <th>Address</th>
              <th>Balance</th>
              <th>Fees Spent</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {wallets.map(wallet => (
              <tr
                key={wallet.worker_id}
                onClick={() => setSelectedWallet(wallet)}
                style={{ cursor: 'pointer' }}
              >
                <td>
                  <strong>kaswallet-{wallet.worker_id}</strong>
                </td>
                <td>
                  {wallet.address ? (
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                      <code style={{ fontSize: '0.75rem', color: '#818cf8', wordBreak: 'break-all' }}>
                        {wallet.address}
                      </code>
                      <button
                        className="btn btn-sm"
                        onClick={(e) => copyAddress(wallet.address, e)}
                        title="Copy address"
                        style={{ padding: '0.25rem 0.5rem', fontSize: '0.75rem' }}
                      >
                        {copiedAddress === wallet.address ? '✓' : '📋'}
                      </button>
                    </div>
                  ) : (
                    <span style={{ color: '#64748b' }}>N/A</span>
                  )}
                </td>
                <td>
                  {wallet.balance !== null && wallet.balance !== undefined ? (
                    <strong>{wallet.balance.toFixed(2)} KAS</strong>
                  ) : (
                    <span style={{ color: '#64748b' }}>N/A</span>
                  )}
                </td>
                <td>
                  {wallet.fees_spent !== null && wallet.fees_spent !== undefined ? (
                    <span style={{ color: '#f59e0b' }}>{wallet.fees_spent.toFixed(4)} KAS</span>
                  ) : (
                    <span style={{ color: '#64748b' }}>N/A</span>
                  )}
                </td>
                <td>{getStatusBadge(wallet.container_running)}</td>
              </tr>
            ))}
          </tbody>
        </table>

        <div style={{ marginTop: '1.5rem', padding: '1rem', background: 'rgba(99, 102, 241, 0.1)', borderRadius: '0.5rem', border: '1px solid #6366f1' }}>
          <p style={{ margin: 0, color: '#818cf8', fontSize: '0.875rem' }}>
            ℹ️ <strong>Tip:</strong> Click on a wallet row to view transaction history. Fees are calculated from initial balance.
          </p>
        </div>
      </div>

      {selectedWallet && (
        <WalletDetailModal
          wallet={selectedWallet}
          onClose={() => setSelectedWallet(null)}
        />
      )}
    </>
  )
}
