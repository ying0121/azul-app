import { apiClient, USE_MOCK } from './client'
import { mockAuth } from './mock'
import { getClinic, getToken } from '@/lib/session'
import type {
  AuthApiResponse,
  AuthCredentials,
  AuthResult,
  AuthStatusResponse,
  Clinic,
} from '@/types/auth'

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function readString(data: Record<string, unknown>, ...keys: string[]): string {
  for (const key of keys) {
    const value = data[key]
    if (typeof value === 'string' && value.trim()) return value.trim()
    if (typeof value === 'number') return String(value)
  }
  return ''
}

function parseClinic(value: unknown): Clinic | null {
  if (value == null) return null
  if (typeof value === 'string' || typeof value === 'number') return value
  if (isRecord(value)) return value as Clinic
  return null
}

function parseAuthResponse(data: unknown): AuthResult {
  if (!isRecord(data)) {
    return { ok: false, message: 'Unable to authenticate. Please try again.' }
  }

  if (data.status === 'success') {
    const token = readString(data, 'token', 'Token')
    const clinic = parseClinic(data.clinic ?? data.Clinic)
    const message = readString(data, 'message', 'Message') || 'Success!'

    if (!token) {
      return { ok: false, message: 'Authentication succeeded but no token was returned.' }
    }

    if (clinic == null) {
      return { ok: false, message: 'Authentication succeeded but no clinic was returned.' }
    }

    return { ok: true, clinic, token, message }
  }

  if (data.status === 'error') {
    return {
      ok: false,
      message: readString(data, 'message', 'Message') || 'Authentication failed.',
    }
  }

  return {
    ok: false,
    message: readString(data, 'message', 'Message') || 'Authentication failed.',
  }
}

export async function authenticate(credentials: AuthCredentials): Promise<AuthResult> {
  if (USE_MOCK) return mockAuth.authenticate(credentials)

  const { data } = await apiClient.post<AuthApiResponse>('/daily-huddle/auth', {
    code: credentials.code.trim(),
  })

  return parseAuthResponse(data)
}

export async function checkAuthStatus(): Promise<AuthStatusResponse> {
  if (USE_MOCK) return mockAuth.checkStatus()

  const token = getToken()
  const clinic = getClinic()

  if (token && clinic != null) {
    return { authenticated: true, clinic }
  }

  return {
    authenticated: false,
    expired: true,
    message: 'Your session has expired. Please sign in again.',
  }
}

export async function logout(): Promise<void> {
  if (USE_MOCK) return mockAuth.logout()
}
