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
    const response = await fetch(`${API_BASE}${endpoint}`, {
      ...options,
      credentials: 'include', // Important: Include cookies for session auth
      headers: {
        'Content-Type': 'application/json',
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

  async getLogRotationConfig() {
    return this.request('/api/storage/log-rotation');
  }

  async updateGlobalLogRotation(settings) {
    return this.request('/api/storage/log-rotation/global', {
      method: 'PUT',
      body: JSON.stringify(settings),
    });
  }

  async getContainerLogRotation(containerName) {
    return this.request(`/api/storage/log-rotation/container/${containerName}`);
  }

  async updateContainerLogRotation(containerName, settings) {
    return this.request(`/api/storage/log-rotation/container/${containerName}`, {
      method: 'PUT',
      body: JSON.stringify(settings),
    });
  }

  async deleteContainerLogRotation(containerName) {
    return this.request(`/api/storage/log-rotation/container/${containerName}`, {
      method: 'DELETE',
    });
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

  // System Services
  async getSystemServices() {
    return this.request('/api/system-services');
  }

  async getAvailableSystemServices() {
    return this.request('/api/system-services/available');
  }

  async getSystemServiceDetails(name) {
    return this.request(`/api/system-services/${encodeURIComponent(name)}/details`);
  }

  async getSystemServiceLogs(name, lines = 100) {
    return this.request(`/api/system-services/${encodeURIComponent(name)}/logs?lines=${lines}`);
  }

  async startSystemService(name) {
    return this.request(`/api/system-services/${encodeURIComponent(name)}/start`, { method: 'POST' });
  }

  async stopSystemService(name) {
    return this.request(`/api/system-services/${encodeURIComponent(name)}/stop`, { method: 'POST' });
  }

  async restartSystemService(name) {
    return this.request(`/api/system-services/${encodeURIComponent(name)}/restart`, { method: 'POST' });
  }

  async enableSystemService(name) {
    return this.request(`/api/system-services/${encodeURIComponent(name)}/enable`, { method: 'POST' });
  }

  async disableSystemService(name) {
    return this.request(`/api/system-services/${encodeURIComponent(name)}/disable`, { method: 'POST' });
  }

  // Categories
  async getCategories() {
    return this.request('/api/categories');
  }

  async getCategory(id) {
    return this.request(`/api/categories/${encodeURIComponent(id)}`);
  }

  async createCategory(category) {
    return this.request('/api/categories', {
      method: 'POST',
      body: JSON.stringify(category),
    });
  }

  async updateCategory(id, category) {
    return this.request(`/api/categories/${encodeURIComponent(id)}`, {
      method: 'PUT',
      body: JSON.stringify(category),
    });
  }

  async deleteCategory(id) {
    return this.request(`/api/categories/${encodeURIComponent(id)}`, { method: 'DELETE' });
  }

  // Tracked Services
  async getTrackedServices() {
    return this.request('/api/tracked-services');
  }

  async updateTrackedService(name, tracked) {
    return this.request(`/api/tracked-services/${encodeURIComponent(name)}`, {
      method: 'PUT',
      body: JSON.stringify(tracked),
    });
  }

  async removeTrackedService(name) {
    return this.request(`/api/tracked-services/${encodeURIComponent(name)}`, { method: 'DELETE' });
  }

  // Authentication (note: session handled automatically via cookies)
  async login(username, password) {
    return this.request('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    });
  }

  async logout() {
    return this.request('/api/auth/logout', { method: 'POST' });
  }

  async getSession() {
    return this.request('/api/auth/session');
  }

  async changePassword(username, currentPassword, newPassword) {
    return this.request('/api/auth/change-password', {
      method: 'POST',
      body: JSON.stringify({
        current_password: currentPassword,
        new_password: newPassword
      }),
    });
  }

  // User Management (admin only)
  async getUsers() {
    return this.request('/api/users');
  }

  async addUser(username, password, roles) {
    return this.request('/api/users', {
      method: 'POST',
      body: JSON.stringify({ username, password, roles }),
    });
  }

  async deleteUser(username) {
    return this.request(`/api/users/${encodeURIComponent(username)}`, { method: 'DELETE' });
  }

  async resetPassword(username, password) {
    return this.request(`/api/users/${encodeURIComponent(username)}/password`, {
      method: 'PUT',
      body: JSON.stringify({ password }),
    });
  }

  async updateUserRoles(username, roles) {
    return this.request(`/api/users/${encodeURIComponent(username)}/roles`, {
      method: 'PUT',
      body: JSON.stringify({ roles }),
    });
  }

  // Security Management (admin only)
  async getSecurityConfig() {
    return this.request('/api/security');
  }

  async addAllowedNetwork(network) {
    return this.request('/api/security/ips', {
      method: 'POST',
      body: JSON.stringify({ network }),
    });
  }

  async removeAllowedNetwork(network) {
    return this.request(`/api/security/ips/${encodeURIComponent(network)}`, { method: 'DELETE' });
  }

  // Audit Logs (admin only)
  async getAuditLogs(limit = 50) {
    return this.request(`/api/audit?limit=${limit}`);
  }

  async exportAuditLogs() {
    return this.request('/api/audit/export');
  }
}

export const api = new IgraApiClient();
