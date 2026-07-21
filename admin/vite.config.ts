import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'

const adminApiTarget = process.env.ADMIN_API_HTTPS || process.env.ADMIN_API_HTTP || 'http://localhost:5047'

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  server: {
    port: 4173,
    proxy: {
      '/api': {
        target: adminApiTarget,
        changeOrigin: true,
        secure: false,
      },
    },
  },
})
