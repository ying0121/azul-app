import { useCallback, useEffect, useRef } from 'react'
import { checkAuthStatus } from '@/api/auth'
import { hasStoredSession, invalidateSession } from '@/lib/sessionGuard'
import { useAuthStore } from '@/stores/authStore'

export function useAuthCheck(enabled = true) {
  const hasInitialized = useRef(false)

  const verifySession = useCallback(async () => {
    const { isAuthenticated, clinic, setClinic, setAuthenticated, setLoading } =
      useAuthStore.getState()

    if (isAuthenticated && clinic != null && hasStoredSession()) {
      setLoading(false)
      return true
    }

    setLoading(true)

    try {
      const status = await checkAuthStatus()

      if (status.authenticated && status.clinic != null) {
        setClinic(status.clinic)
        setAuthenticated(true)
        return true
      }

      if (status.expired) {
        invalidateSession({ message: status.message })
        return false
      }

      invalidateSession({
        message: status.message ?? 'Please sign in to continue.',
        showAlert: false,
      })
      return false
    } catch (err: unknown) {
      const message =
        (err as { friendlyMessage?: string }).friendlyMessage ??
        'Unable to verify your session.'
      invalidateSession({ message })
      return false
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    if (!enabled || hasInitialized.current) return
    hasInitialized.current = true
    void verifySession()
  }, [enabled, verifySession])

  return { verifySession }
}
