/** Shared API URL helpers for Vite config. Keep in sync with src/lib/apiConfig.ts */

export function normalizeApiUrl(url: string | undefined): string {
  return String(url ?? '')
    .trim()
    .replace(/\/$/, '')
    .replace(/\/\/localhost\b/i, '//127.0.0.1')
}

/** Vite dev-server proxy target — reads VITE_API_PROXY_TARGET, then VITE_API_BASE_URL from .env */
export function resolveProxyTarget(env: Record<string, string>): string {
  const proxyTarget = normalizeApiUrl(env.VITE_API_PROXY_TARGET)
  if (proxyTarget) return proxyTarget

  const baseUrl = normalizeApiUrl(env.VITE_API_BASE_URL)
  if (baseUrl) return baseUrl

  return 'http://127.0.0.1:5005'
}
