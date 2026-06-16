import type { Clinic } from '@/types/auth'

const TOKEN_KEY = 'dh_token'
const HUDDLE_TOKEN_KEY = 'dh_huddle_token'
const CLINIC_KEY = 'dh_clinic'

export function saveToken(token: string) {
  sessionStorage.setItem(TOKEN_KEY, token.trim())
}

export function getToken(): string | null {
  return sessionStorage.getItem(TOKEN_KEY)
}

export function clearToken() {
  sessionStorage.removeItem(TOKEN_KEY)
}

export function saveHuddleToken(token: string) {
  sessionStorage.setItem(HUDDLE_TOKEN_KEY, token.trim())
}

export function getHuddleToken(): string | null {
  return sessionStorage.getItem(HUDDLE_TOKEN_KEY)
}

export function clearHuddleToken() {
  sessionStorage.removeItem(HUDDLE_TOKEN_KEY)
}

export function saveClinic(clinic: Clinic) {
  sessionStorage.setItem(CLINIC_KEY, JSON.stringify(clinic))
}

export function getClinic(): Clinic | null {
  const raw = sessionStorage.getItem(CLINIC_KEY)
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
  sessionStorage.removeItem(CLINIC_KEY)
}
