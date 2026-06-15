import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'node:path'
import { resolveProxyTarget } from './scripts/apiEnv.ts'

const host = process.env.TAURI_DEV_HOST

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const apiTarget = resolveProxyTarget(env)

  if (mode === 'development') {
    console.info(`[vite] API proxy /daily-huddle → ${apiTarget}`)
  }

  return {
    plugins: [react()],
    base: './',
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_ENV_*', 'TAURI_*'],
    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
      },
    },
    build: {
      minify: true,
      sourcemap: false,
      target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    },
    server: {
      port: 5173,
      strictPort: true,
      // Use 127.0.0.1 so Tauri's dev-server probe works on Windows (avoids ::1 vs localhost mismatch).
      host: host || '127.0.0.1',
      hmr: host
        ? { protocol: 'ws', host, port: 1421 }
        : undefined,
      watch: {
        ignored: ['**/src-tauri/**', '**/release/**', '**/release-build/**', '**/dist-release/**'],
      },
      proxy: {
        '/daily-huddle': {
          target: apiTarget,
          changeOrigin: true,
          secure: apiTarget.startsWith('https://'),
        },
      },
    },
  }
})
