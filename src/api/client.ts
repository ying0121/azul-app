import axios, { AxiosError, type InternalAxiosRequestConfig } from 'axios'
import { getApiBaseUrl, getApiConfigLabel } from '@/lib/apiConfig'
import { invalidateSession, isAuthApiRequest } from '@/lib/sessionGuard'
import { clearToken, getToken, saveToken } from '@/lib/session'

export const USE_MOCK = import.meta.env.VITE_USE_MOCK === 'true'

export const apiClient = axios.create({
  baseURL: getApiBaseUrl(),
  withCredentials: false,
  headers: {
    'Content-Type': 'application/json',
  },
  timeout: 15000,
})

if (import.meta.env.DEV) {
  console.info(`[api] ${getApiConfigLabel()}`)
}

/** Apply token to local storage and every axios request (Authorization: <token>) */
export function setAuthToken(token: string | null) {
  if (token) {
    const value = token.trim()
    saveToken(value)
    apiClient.defaults.headers.common.Authorization = value
  } else {
    clearToken()
    delete apiClient.defaults.headers.common.Authorization
  }
}

function attachAuthorizationHeader(config: InternalAxiosRequestConfig) {
  const url = config.url ?? ''

  if (!isAuthApiRequest(url) && !getToken()) {
    invalidateSession({
      message: 'Your session has expired. Please sign in again.',
    })
    return Promise.reject(
      Object.assign(new Error('Session expired'), {
        code: 'ERR_SESSION_EXPIRED',
        config,
      }),
    )
  }

  const token = getToken()
  if (!token) return config

  if (config.headers && typeof config.headers.set === 'function') {
    config.headers.set('Authorization', token)
  } else {
    config.headers = config.headers ?? {}
    ;(config.headers as Record<string, string>).Authorization = token
  }

  return config
}

apiClient.interceptors.request.use(attachAuthorizationHeader)

apiClient.interceptors.response.use(
  (response) => response,
  (error: AxiosError<{ status?: string; message?: string }>) => {
    let message = error.response?.data?.message

    if (!message) {
      if (error.response?.status === 401) {
        message = 'Your session has expired. Please log in again.'
      } else if (error.code === 'ERR_NETWORK' || !error.response) {
        message = `Unable to reach the API server (${getApiConfigLabel()}). Check VITE_API_BASE_URL and VITE_API_PROXY_TARGET in .env, then rebuild for production.`
      } else {
        message = 'An unexpected error occurred. Please try again.'
      }
    }

    if (error.response?.status === 401 && !isAuthApiRequest(error.config?.url)) {
      invalidateSession({ message })
    }

    return Promise.reject({ ...error, friendlyMessage: message })
  },
)
