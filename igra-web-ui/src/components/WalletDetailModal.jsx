import { useState, useEffect } from 'react'
import { api } from '../services/api'

export default function WalletDetailModal({ wallet, onClose }) {
  const [utxos, setUtxos] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)

  useEffect(() => {
    loadTransactions()
  }, [wallet.worker_id])

  async function loadTransactions() {
    try {
      const data = await api.getWalletDetail(wallet.worker_id)
      // Sort by timestamp descending (most recent first)
      const sorted = data.sort((a, b) => b.timestamp_ms - a.timestamp_ms)
      setUtxos(sorted)
      setError(null)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  function formatTimestamp(timestampMs) {
    const date = new Date(timestampMs)
    return date.toLocaleString()
  }

  function formatAddress(address) {
    if (!address || address.length < 20) return address
    return `${address.substring(0, 15)}...${address.substring(address.length - 10)}`
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content modal-large" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Wallet kaswallet-{wallet.worker_id} - Transaction History</h2>
          <button className="btn-close" onClick={onClose}>×</button>
        </div>

        <div style={{ padding: '1rem', borderBottom: '1px solid #374151' }}>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1rem' }}>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#9ca3af', marginBottom: '0.25rem' }}>Current Balance</div>
              <div style={{ fontSize: '1.25rem', fontWeight: 'bold', color: '#10b981' }}>
                {wallet.balance !== null && wallet.balance !== undefined ? `${wallet.balance.toFixed(2)} KAS` : 'N/A'}
              </div>
            </div>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#9ca3af', marginBottom: '0.25rem' }}>Initial Balance</div>
              <div style={{ fontSize: '1.25rem', fontWeight: 'bold', color: '#818cf8' }}>
                {wallet.initial_balance !== null && wallet.initial_balance !== undefined ? `${wallet.initial_balance.toFixed(2)} KAS` : 'N/A'}
              </div>
            </div>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#9ca3af', marginBottom: '0.25rem' }}>Fees Spent</div>
              <div style={{ fontSize: '1.25rem', fontWeight: 'bold', color: '#f59e0b' }}>
                {wallet.fees_spent !== null && wallet.fees_spent !== undefined ? `${wallet.fees_spent.toFixed(4)} KAS` : 'N/A'}
              </div>
            </div>
            <div>
              <div style={{ fontSize: '0.75rem', color: '#9ca3af', marginBottom: '0.25rem' }}>Total Transactions</div>
              <div style={{ fontSize: '1.25rem', fontWeight: 'bold' }}>
                {loading ? '...' : utxos.length}
              </div>
            </div>
          </div>
        </div>

        <div style={{ padding: '1rem', maxHeight: '60vh', overflowY: 'auto' }}>
          {loading && <div style={{ textAlign: 'center', color: '#64748b' }}>Loading transactions...</div>}
          {error && <div className="error">Error: {error}</div>}

          {!loading && !error && utxos.length === 0 && (
            <div style={{ textAlign: 'center', color: '#64748b', padding: '2rem' }}>
              No transactions found
            </div>
          )}

          {!loading && !error && utxos.length > 0 && (
            <table className="table">
              <thead>
                <tr>
                  <th>Date</th>
                  <th>Amount</th>
                  <th>Type</th>
                  <th>TX ID</th>
                  <th>DAA Score</th>
                  <th>From</th>
                </tr>
              </thead>
              <tbody>
                {utxos.map((utxo, idx) => (
                  <tr key={idx}>
                    <td style={{ fontSize: '0.75rem' }}>
                      {formatTimestamp(utxo.timestamp_ms)}
                    </td>
                    <td>
                      <strong style={{ color: '#10b981' }}>
                        {utxo.amount_kas.toFixed(2)} KAS
                      </strong>
                    </td>
                    <td>
                      {utxo.is_coinbase ? (
                        <span className="badge badge-warning">Coinbase</span>
                      ) : (
                        <span className="badge badge-info">Transfer</span>
                      )}
                    </td>
                    <td>
                      <code style={{ fontSize: '0.7rem', color: '#818cf8' }}>
                        {formatAddress(utxo.tx_id)}
                      </code>
                    </td>
                    <td style={{ fontSize: '0.875rem', color: '#9ca3af' }}>
                      {utxo.block_daa_score.toLocaleString()}
                    </td>
                    <td>
                      {utxo.is_coinbase ? (
                        <span style={{ fontSize: '0.75rem', color: '#64748b' }}>Mining Reward</span>
                      ) : utxo.source_addresses && utxo.source_addresses.length > 0 ? (
                        <code style={{ fontSize: '0.7rem', color: '#818cf8' }}>
                          {formatAddress(utxo.source_addresses[0])}
                          {utxo.source_addresses.length > 1 && ` +${utxo.source_addresses.length - 1}`}
                        </code>
                      ) : (
                        <span style={{ fontSize: '0.75rem', color: '#64748b' }}>Unknown</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        <div className="modal-footer">
          <button className="btn" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  )
}
