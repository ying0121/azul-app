import { getClinic, getToken } from '@/lib/session'
import { useAlertStore } from '@/stores/alertStore'
import { useAuthStore } from '@/stores/authStore'

type NavigateFn = (path: string, options?: { replace?: boolean }) => void

let navigateFn: NavigateFn | null = null
let isInvalidating = false

export function registerAuthNavigator(fn: NavigateFn) {
  navigateFn = fn
}

export function redirectToAuth(replace = true) {
  if (navigateFn) {
    navigateFn('/auth', { replace })
    return
  }
  window.location.hash = '#/auth'
}

export function hasStoredSession(): boolean {
  return getToken() != null && getClinic() != null
}

export function invalidateSession(options?: {
  message?: string
  showAlert?: boolean
  title?: string
}) {
  if (isInvalidating) return
  isInvalidating = true

  useAuthStore.getState().reset()

  if (options?.showAlert !== false) {
    useAlertStore.getState().show(
      'error',
      options?.title ?? 'Session Expired',
      options?.message ?? 'Your session has expired. Please sign in again.',
    )
  }

  redirectToAuth(true)

  queueMicrotask(() => {
    isInvalidating = false
  })
}

export function isAuthApiRequest(url: string | undefined): boolean {
  return (url ?? '').includes('/daily-huddle/auth')
}
