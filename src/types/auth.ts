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
