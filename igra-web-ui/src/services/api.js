/**
 * IGRA API Client
 * Communicates with igra-cli HTTP server
 */

// Use relative URL so it works from any host (localhost or IP address)
const API_BASE = import.meta.env.VITE_API_URL || '';

class IgraApiClient {
  getToken() {
    return localStorage.getItem('igra_token') || '';
  }

  setToken(token) {
    localStorage.setItem('igra_token', token);
  }

  clearToken() {
    localStorage.removeItem('igra_token');
  }

  async request(endpoint, options = {}) {
    const token = this.getToken();

    const response = await fetch(`${API_BASE}${endpoint}`, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...(token && { 'Authorization': `Bearer ${token}` }),
        ...options.headers,
      },
    });

    if (response.status === 401) {
      // Unauthorized - clear token and throw
      this.clearToken();
      throw new Error('Unauthorized - please login again');
    }

    if (!response.ok) {
      throw new Error(`API error: ${response.statusText}`);
    }

    const data = await response.json();

    // Handle both wrapped ({success, data}) and unwrapped responses
    if (data.success !== undefined) {
      if (!data.success) {
        throw new Error(data.error || 'Unknown error');
      }
      return data.data;
    }

    // If not wrapped, return data directly
    return data;
  }

  // Service Management
  async getServices(options = {}) {
    const query = new URLSearchParams();

    // Legacy support for showAll parameter
    if (typeof options === 'boolean') {
      if (options) query.append('show_all', 'true');
    } else {
      // New filter parameters
      if (options.showAll) query.append('show_all', 'true');
      if (options.profiles && options.profiles.length > 0) {
        query.append('profiles', options.profiles.join(','));
      }
      if (options.statuses && options.statuses.length > 0) {
        query.append('statuses', options.statuses.join(','));
      }
      if (options.project) {
        query.append('project', options.project);
      }
      if (options.name) {
        query.append('name', options.name);
      }
    }

    const queryString = query.toString();
    return this.request(`/api/services${queryString ? '?' + queryString : ''}`);
  }

  async startService(name) {
    return this.request(`/api/services/${name}/start`, { method: 'POST' });
  }

  async stopService(name) {
    return this.request(`/api/services/${name}/stop`, { method: 'POST' });
  }

  async restartService(name) {
    return this.request(`/api/services/${name}/restart`, { method: 'POST' });
  }

  async getServiceLogs(name, tail = 100) {
    return this.request(`/api/services/${name}/logs?tail=${tail}`);
  }

  async getServiceDetails(name) {
    return this.request(`/api/services/${name}/details`);
  }

  // Wallet Management
  async getWallets() {
    return this.request('/api/wallets');
  }

  async getWalletBalance(id) {
    return this.request(`/api/wallets/${id}/balance`);
  }

  async getWalletDetail(id) {
    return this.request(`/api/wallets/${id}/detail`);
  }

  // Storage
  async getStorage() {
    return this.request('/api/storage');
  }

  async getStorageHistory() {
    return this.request('/api/storage/history');
  }

  async pruneStorage() {
    return this.request('/api/storage/prune', { method: 'POST' });
  }

  async truncateContainerLog(containerId) {
    return this.request(`/api/storage/container-logs/${containerId}/truncate`, { method: 'POST' });
  }

  // Configuration
  async getConfig() {
    return this.request('/api/config');
  }

  async getSystemInfo() {
    return this.request('/api/system');
  }

  async getRpcTokens() {
    return this.request('/api/rpc/tokens');
  }

  async getSslInfo() {
    return this.request('/api/ssl/info');
  }

  // Health
  async getHealth() {
    return this.request('/api/health');
  }

  // Parsed Logs
  async getServiceLogsParsed(name, params = {}) {
    const query = new URLSearchParams();
    if (params.tail) query.append('tail', params.tail);
    if (params.level) query.append('level', params.level);
    if (params.module) query.append('module', params.module);
    return this.request(`/api/services/${name}/logs/parsed?${query}`);
  }

  // Profiles
  async getProfiles() {
    return this.request('/api/profiles');
  }

  async startProfile(name) {
    return this.request(`/api/profiles/${name}/start`, { method: 'POST' });
  }

  async stopProfile(name) {
    return this.request(`/api/profiles/${name}/stop`, { method: 'POST' });
  }

  // Transactions
  async getTransactions(params = {}) {
    const query = new URLSearchParams();
    if (params.limit) query.append('limit', params.limit);
    if (params.filter) query.append('filter', params.filter);
    return this.request(`/api/transactions?${query}`);
  }

  async getTransactionStats() {
    return this.request('/api/transactions/stats');
  }

  // WebSocket connections
  connectLogsWebSocket(serviceName, onMessage) {
    // Use current host for WebSocket connection
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsHost = window.location.host;
    const ws = new WebSocket(`${wsProtocol}//${wsHost}/ws/logs/${serviceName}`);

    ws.onmessage = (event) => {
      try {
        const logs = JSON.parse(event.data);
        onMessage(logs);
      } catch (error) {
        console.error('Error parsing log message:', error);
      }
    };

    return ws;
  }

  connectMetricsWebSocket(onMessage) {
    // Use current host for WebSocket connection
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsHost = window.location.host;
    const ws = new WebSocket(`${wsProtocol}//${wsHost}/ws/metrics`);

    ws.onmessage = (event) => {
      try {
        const metrics = JSON.parse(event.data);
        onMessage(metrics);
      } catch (error) {
        console.error('Error parsing metrics message:', error);
      }
    };

    return ws;
  }

  // Service Management
  async restartService() {
    return this.request('/api/service/restart', { method: 'POST' });
  }
}

export const api = new IgraApiClient();
