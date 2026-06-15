import { Navigate } from 'react-router-dom'
import { useAuthStore } from '@/stores/authStore'
import { useAuthCheck } from '@/hooks/useAuthCheck'

interface ProtectedRouteProps {
  children: React.ReactNode
}

export function ProtectedRoute({ children }: ProtectedRouteProps) {
  const { isAuthenticated, isLoading } = useAuthStore()
  useAuthCheck(true)

  if (isLoading) {
    return (
      <div className="app-loading">
        <div className="spinner" />
        <p>Checking session...</p>
      </div>
    )
  }

  if (!isAuthenticated) {
    return <Navigate to="/auth" replace />
  }

  return <>{children}</>
}
