import type { Clinic } from '@/types/auth'

const TOKEN_KEY = 'dh_token'
const HUDDLE_TOKEN_KEY = 'dh_huddle_token'
const CLINIC_KEY = 'dh_clinic'
const SESSION_EXPIRY_KEY = 'dh_session_expires_at'

export const SESSION_DURATION_MS = 7 * 24 * 60 * 60 * 1000

function storage() {
  return localStorage
}

function isSessionExpired(): boolean {
  const raw = storage().getItem(SESSION_EXPIRY_KEY)
  if (!raw) return true
  const expiresAt = Number(raw)
  return !Number.isFinite(expiresAt) || Date.now() >= expiresAt
}

function clearExpiredSession() {
  if (!isSessionExpired()) return
  clearSession()
}

export function saveSessionExpiry() {
  storage().setItem(SESSION_EXPIRY_KEY, String(Date.now() + SESSION_DURATION_MS))
}

export function saveToken(token: string) {
  storage().setItem(TOKEN_KEY, token.trim())
}

export function getToken(): string | null {
  clearExpiredSession()
  return storage().getItem(TOKEN_KEY)
}

export function clearToken() {
  storage().removeItem(TOKEN_KEY)
}

export function saveHuddleToken(token: string) {
  storage().setItem(HUDDLE_TOKEN_KEY, token.trim())
}

export function getHuddleToken(): string | null {
  clearExpiredSession()
  return storage().getItem(HUDDLE_TOKEN_KEY)
}

export function clearHuddleToken() {
  storage().removeItem(HUDDLE_TOKEN_KEY)
}

export function saveClinic(clinic: Clinic) {
  storage().setItem(CLINIC_KEY, JSON.stringify(clinic))
}

export function getClinic(): Clinic | null {
  clearExpiredSession()
  const raw = storage().getItem(CLINIC_KEY)
  if (!raw) return null
  try {
    return JSON.parse(raw) as Clinic
  } catch {
    return raw
  }
}

export function clearSession() {
  clearToken()
  clearHuddleToken()
  storage().removeItem(CLINIC_KEY)
  storage().removeItem(SESSION_EXPIRY_KEY)
}
