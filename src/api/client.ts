import axios, { AxiosError, type InternalAxiosRequestConfig } from 'axios'
import { clearToken, getToken, saveToken } from '@/lib/session'

const configuredBaseUrl = import.meta.env.VITE_API_BASE_URL
const API_BASE_URL = import.meta.env.DEV
  ? ''
  : configuredBaseUrl != null && String(configuredBaseUrl).trim() !== ''
    ? String(configuredBaseUrl).replace(/\/$/, '').replace(/\/\/localhost\b/i, '//127.0.0.1')
    : ''
export const USE_MOCK = import.meta.env.VITE_USE_MOCK === 'true'

export const apiClient = axios.create({
  baseURL: API_BASE_URL,
  withCredentials: true,
  headers: {
    'Content-Type': 'application/json',
  },
  timeout: 15000,
})

/** Apply token to session storage and every axios request (Authorization: <token>) */
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
        message =
          'Unable to reach the API server. Make sure the backend is running and check VITE_API_BASE_URL in .env.'
      } else {
        message = 'An unexpected error occurred. Please try again.'
      }
    }

    return Promise.reject({ ...error, friendlyMessage: message })
  },
)
