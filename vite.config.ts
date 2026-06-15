import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'node:path'

const host = process.env.TAURI_DEV_HOST

function resolveApiTarget(env: Record<string, string>): string {
  const raw =
    env.VITE_API_PROXY_TARGET ??
    env.VITE_API_BASE_URL ??
    'http://127.0.0.1:5005'

  return raw.replace(/\/$/, '').replace(/\/\/localhost\b/i, '//127.0.0.1')
}

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const apiTarget = resolveApiTarget(env)

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
        },
      },
    },
  }
})
