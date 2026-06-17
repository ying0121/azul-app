export type Clinic = string | number | Record<string, unknown>

export interface AuthCredentials {
  code: string
}

export interface AuthApiSuccessResponse {
  status: 'success'
  clinic: Clinic
  message: string
  token: string
}

export interface AuthApiErrorResponse {
  status: 'error'
  clinic: null
  message: string
  token: null
}

export type AuthApiResponse = AuthApiSuccessResponse | AuthApiErrorResponse

export type AuthResult =
  | { ok: true; clinic: Clinic; token: string; message: string }
  | { ok: false; message: string }

export interface AuthStatusResponse {
  authenticated: boolean
  expired?: boolean
  clinic?: Clinic | null
  message?: string
}

export function getClinicId(clinic: Clinic | null | undefined): string | number | null {
  if (clinic == null) return null

  if (typeof clinic === 'string' || typeof clinic === 'number') {
    return clinic
  }

  for (const key of ['clinic_id', 'clinicId', 'ClinicId', 'id', 'Id']) {
    const value = clinic[key]
    if (typeof value === 'string' && value.trim()) return value.trim()
    if (typeof value === 'number' && !Number.isNaN(value)) return value
  }

  return null
}

export function getClinicDisplayName(clinic: Clinic | null | undefined): string {
  if (clinic == null) return 'Clinic'

  if (typeof clinic === 'string' || typeof clinic === 'number') {
    return String(clinic)
  }

  for (const key of ['name', 'clinic_name', 'ClinicName', 'clinic', 'title']) {
    const value = clinic[key]
    if (typeof value === 'string' && value.trim()) return value.trim()
  }

  return 'Clinic'
}

const CLINIC_ACRONYM_SKIP = new Set([
  'a',
  'an',
  'the',
  'and',
  'of',
  'md',
  'do',
  'pc',
  'pllc',
  'llc',
  'inc',
  'pa',
  'dr',
  'mr',
  'mrs',
  'ms',
])

export function getClinicAcronym(clinic: Clinic | null | undefined): string {
  if (clinic == null) return 'CL'

  if (typeof clinic === 'object') {
    for (const key of [
      'acronym',
      'clinic_acronym',
      'ClinicAcronym',
      'initials',
      'clinic_initials',
    ]) {
      const value = clinic[key]
      if (typeof value === 'string' && value.trim()) {
        return value.trim().slice(0, 3).toUpperCase()
      }
    }
  }

  const name = getClinicDisplayName(clinic)
  const words = name
    .split(/\s+/)
    .map((word) => word.replace(/[.,]/g, ''))
    .filter((word) => {
      const normalized = word.toLowerCase()
      return word.length > 1 && !CLINIC_ACRONYM_SKIP.has(normalized)
    })

  if (words.length === 0) {
    const fallback = name.replace(/[^a-zA-Z0-9]/g, '').slice(0, 2).toUpperCase()
    return fallback || 'CL'
  }

  if (words.length === 1) {
    return words[0].slice(0, 3).toUpperCase()
  }

  return words
    .map((word) => word[0])
    .join('')
    .slice(0, 3)
    .toUpperCase()
}
