import { useCallback, useEffect, useRef } from 'react'
import { useNavigate } from 'react-router-dom'
import { checkAuthStatus } from '@/api/auth'
import { getToken } from '@/lib/session'
import { useAuthStore } from '@/stores/authStore'
import { useAlertStore } from '@/stores/alertStore'

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

  const verifySession = useCallback(async () => {
    const { isAuthenticated, clinic, setClinic, setAuthenticated, setLoading } =
      useAuthStore.getState()

    if (isAuthenticated && clinic != null && getToken()) {
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
        handleExpired(status.message)
        return false
      }

      useAuthStore.getState().reset()
      return false
    } catch (err: unknown) {
      const message =
        (err as { friendlyMessage?: string }).friendlyMessage ??
        'Unable to verify your session.'
      handleExpired(message)
      return false
    } finally {
      setLoading(false)
    }
  }, [handleExpired])

  useEffect(() => {
    if (!enabled || hasInitialized.current) return
    hasInitialized.current = true
    void verifySession()
  }, [enabled, verifySession])

  return { verifySession, handleExpired }
}
