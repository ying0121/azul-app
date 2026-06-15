import { useCallback, useEffect, useRef } from 'react'
import { useNavigate } from 'react-router-dom'
import { checkAuthStatus } from '@/api/auth'
import { useAuthStore } from '@/stores/authStore'
import { useAlertStore } from '@/stores/alertStore'
import { getClinicId } from '@/types/auth'

const AUTH_CHECK_INTERVAL = 60_000

function clinicIdsMatch(
  a: ReturnType<typeof useAuthStore.getState>['clinic'],
  b: ReturnType<typeof useAuthStore.getState>['clinic'],
): boolean {
  const idA = getClinicId(a)
  const idB = getClinicId(b)
  if (idA == null || idB == null) return idA === idB
  return String(idA) === String(idB)
}

export function useAuthCheck(enabled = true) {
  const navigate = useNavigate()
  const showAlert = useAlertStore((s) => s.show)
  const showAlertRef = useRef(showAlert)
  const hasInitialized = useRef(false)

  showAlertRef.current = showAlert

  const handleExpired = useCallback((message?: string) => {
    useAuthStore.getState().reset()
    showAlertRef.current(
      'error',
      'Session Expired',
      message ?? 'Your session has expired. Please sign in again.',
    )
    navigate('/auth', { replace: true })
  }, [navigate])

  const verifySession = useCallback(
    async (silent = false) => {
      const { setClinic, setAuthenticated, setLoading, isAuthenticated, clinic } =
        useAuthStore.getState()

      if (!silent) setLoading(true)

      try {
        const status = await checkAuthStatus()

        if (status.authenticated && status.clinic != null) {
          if (!isAuthenticated || !clinicIdsMatch(clinic, status.clinic)) {
            setClinic(status.clinic)
            setAuthenticated(true)
          }
          return true
        }

        if (status.expired) {
          handleExpired(status.message)
          return false
        }

        if (isAuthenticated) {
          useAuthStore.getState().reset()
        }
        return false
      } catch (err: unknown) {
        const message =
          (err as { friendlyMessage?: string }).friendlyMessage ??
          'Unable to verify your session.'
        handleExpired(message)
        return false
      } finally {
        if (!silent) setLoading(false)
      }
    },
    [handleExpired],
  )

  useEffect(() => {
    if (!enabled) return

    if (!hasInitialized.current) {
      hasInitialized.current = true
      void verifySession(false)
    }

    const interval = setInterval(() => {
      void verifySession(true)
    }, AUTH_CHECK_INTERVAL)

    return () => clearInterval(interval)
  }, [enabled, verifySession])

  return { verifySession, handleExpired }
}
