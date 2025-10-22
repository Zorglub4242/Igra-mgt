import { useState, useEffect } from 'react'
import { api } from '../services/api'

export default function TransactionsPanel() {
  const [transactions, setTransactions] = useState([])
  const [stats, setStats] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [filter, setFilter] = useState('all')

  useEffect(() => {
    loadData()
    const interval = setInterval(loadData, 2000) // Refresh every 2s for real-time feel
    return () => clearInterval(interval)
  }, [filter])

  async function loadData() {
    try {
      const [txData, statsData] = await Promise.all([
        api.getTransactions({ limit: 50, filter }),
        api.getTransactionStats()
      ])
      setTransactions(txData)
      setStats(statsData)
      setError(null)
    } catch (err) {
      setError(err.message)
    } finally {
      setLoading(false)
    }
  }

  function getTxTypeColor(txType) {
    if (txType.includes('Entry')) return '#818cf8'
    if (txType.includes('Contract')) return '#f59e0b'
    if (txType.includes('Transfer')) return '#10b981'
    return '#94a3b8'
  }

  function formatAddress(addr) {
    if (!addr) return 'N/A'
    return `${addr.slice(0, 6)}...${addr.slice(-4)}`
  }

  function formatValue(val) {
    return val.toFixed(4)
  }

  if (loading && !stats) {
    return <div className="loading">Loading transactions...</div>
  }

  return (
    <div>
      {/* Statistics Cards */}
      {stats && (
        <div className="stats-grid">
          <div className="stat-card">
            <div className="stat-label">Current Block</div>
            <div className="stat-value">#{stats.current_block.toLocaleString()}</div>
          </div>
          <div className="stat-card">
            <div className="stat-label">TPS</div>
            <div className="stat-value">{stats.tps.toFixed(2)}</div>
          </div>
          <div className="stat-card">
            <div className="stat-label">Total Transactions</div>
            <div className="stat-value">{stats.total_transactions.toLocaleString()}</div>
          </div>
          <div className="stat-card">
            <div className="stat-label">Success Rate</div>
            <div className="stat-value">
              {stats.total_transactions > 0
                ? ((stats.successful_transactions / stats.total_transactions) * 100).toFixed(1)
                : 0}%
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-label">Gas Fees (iKAS)</div>
            <div className="stat-value">{stats.total_gas_fees_ikas.toFixed(2)}</div>
          </div>
          <div className="stat-card">
            <div className="stat-label">L1 Fees (KAS)</div>
            <div className="stat-value">{stats.total_l1_fees_kas.toFixed(2)}</div>
          </div>
        </div>
      )}

      {/* Transaction List */}
      <div className="card" style={{ marginTop: '1.5rem' }}>
        <div className="card-header">
          <h2 className="card-title">Recent Transactions</h2>
          <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
            <select value={filter} onChange={(e) => setFilter(e.target.value)} className="filter-select">
              <option value="all">All Types</option>
              <option value="transfer">Transfers</option>
              <option value="contract">Contracts</option>
              <option value="entry">Entry Txs</option>
            </select>
            <button className="btn" onClick={loadData}>
              🔄 Refresh
            </button>
          </div>
        </div>

        {error && <div className="error">Error: {error}</div>}

        {transactions.length === 0 ? (
          <div style={{ padding: '2rem', textAlign: 'center', color: '#64748b' }}>
            No transactions found
          </div>
        ) : (
          <div className="tx-list">
            {transactions.map(tx => (
              <div key={tx.hash} className="tx-item">
                <div className="tx-row">
                  <div className="tx-hash">
                    <strong>Hash:</strong> {formatAddress(tx.hash)}
                  </div>
                  <span
                    className="tx-type-badge"
                    style={{ background: getTxTypeColor(tx.tx_type), color: 'white' }}
                  >
                    {tx.tx_type}
                  </span>
                  {tx.status ? (
                    <span className="badge badge-success">✓ Success</span>
                  ) : (
                    <span className="badge badge-danger">✗ Failed</span>
                  )}
                </div>

                <div className="tx-details">
                  <div><strong>From:</strong> {formatAddress(tx.from)}</div>
                  <div><strong>To:</strong> {tx.to ? formatAddress(tx.to) : 'Contract Creation'}</div>
                  <div><strong>Value:</strong> {formatValue(tx.value_ikas)} iKAS</div>
                  <div><strong>Gas Fee:</strong> {formatValue(tx.gas_fee_ikas)} iKAS</div>
                  {tx.l1_fee_kas && (
                    <div><strong>L1 Fee:</strong> {formatValue(tx.l1_fee_kas)} KAS</div>
                  )}
                  <div><strong>Block:</strong> #{tx.block_number}</div>
                  <div><strong>Time:</strong> {new Date(tx.timestamp).toLocaleString()}</div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
