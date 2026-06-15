/** API URL resolution — values come from .env (see .env.example). */

export function normalizeApiUrl(url: string | undefined): string {
  return String(url ?? '')
    .trim()
    .replace(/\/$/, '')
    .replace(/\/\/localhost\b/i, '//127.0.0.1')
}

/** True in dev when .env.development sets VITE_API_USE_PROXY=true (routes via Vite proxy). */
export function isApiProxyEnabled(): boolean {
  return import.meta.env.VITE_API_USE_PROXY === 'true'
}

export function getConfiguredApiBaseUrl(): string {
  return normalizeApiUrl(import.meta.env.VITE_API_BASE_URL)
}

export function getConfiguredApiProxyTarget(): string {
  const proxyTarget = normalizeApiUrl(import.meta.env.VITE_API_PROXY_TARGET)
  if (proxyTarget) return proxyTarget
  return getConfiguredApiBaseUrl()
}

/**
 * Axios base URL.
 * - Dev + proxy: '' (requests hit /daily-huddle on the Vite server → proxied to VITE_API_PROXY_TARGET)
 * - Production / direct: VITE_API_BASE_URL from .env (baked in at build time)
 */
export function getApiBaseUrl(): string {
  if (isApiProxyEnabled()) return ''
  return getConfiguredApiBaseUrl()
}

export function getApiConfigLabel(): string {
  if (isApiProxyEnabled()) {
    const target = getConfiguredApiProxyTarget()
    return target ? `proxy → ${target}` : 'proxy (set VITE_API_PROXY_TARGET in .env)'
  }
  const base = getConfiguredApiBaseUrl()
  return base || 'VITE_API_BASE_URL is not set in .env'
}
