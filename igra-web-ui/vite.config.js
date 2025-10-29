import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    host: '0.0.0.0', // Allow access from other devices on LAN
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:3000', // Production backend port
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://localhost:3000', // Production backend WebSocket port
        ws: true,
      }
    }
  },
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
  }
})
