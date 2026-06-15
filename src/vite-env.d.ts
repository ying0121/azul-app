/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL: string
  readonly VITE_API_PROXY_TARGET: string
  readonly VITE_API_USE_PROXY: string
  readonly VITE_USE_MOCK: string
  readonly VITE_CLINIC_TIMEZONE: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}
