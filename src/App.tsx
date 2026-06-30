import { useEffect } from 'react'
import { HashRouter, Navigate, Route, Routes, useNavigate } from 'react-router-dom'
import { registerAuthNavigator } from '@/lib/sessionGuard'
import { ToastHost } from '@/components/ui/ToastHost'
import { TitleBar } from '@/components/layout/TitleBar'
import { useDesktopRestrictions } from '@/hooks/useDesktopRestrictions'
import { isDesktopApp } from '@/lib/tauri'
import { AuthPage } from '@/pages/AuthPage'
import { DashboardPage } from '@/pages/DashboardPage'
import { ProtectedRoute } from '@/routes/ProtectedRoute'
import { PublicRoute } from '@/routes/PublicRoute'
import { useAuthStore } from '@/stores/authStore'
import { useStatusColorStore } from '@/stores/statusColorStore'
import { useThemeStore } from '@/stores/themeStore'

function AppRoutes() {
  const navigate = useNavigate()
  const hydrateFromSession = useAuthStore((s) => s.hydrateFromSession)
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)

  useEffect(() => {
    registerAuthNavigator((path, options) => {
      navigate(path, options)
    })
    hydrateFromSession()
  }, [hydrateFromSession, navigate])

  useEffect(() => {
    if (!isAuthenticated) return
    void useStatusColorStore.getState().loadStatusColors()
  }, [isAuthenticated])

  return (
    <Routes>
      <Route path="/" element={<Navigate to="/auth" replace />} />
      <Route
        path="/auth"
        element={
          <PublicRoute>
            <AuthPage />
          </PublicRoute>
        }
      />
      <Route
        path="/dashboard"
        element={
          <ProtectedRoute>
            <DashboardPage />
          </ProtectedRoute>
        }
      />
      <Route path="*" element={<Navigate to="/auth" replace />} />
    </Routes>
  )
}

export default function App() {
  const desktop = isDesktopApp()
  const hydrateTheme = useThemeStore((s) => s.hydrateTheme)

  useDesktopRestrictions()

  useEffect(() => {
    hydrateTheme()
  }, [hydrateTheme])

  return (
    <HashRouter>
      <div className={desktop ? 'app-shell app-shell--desktop' : 'app-shell'}>
        {desktop && <TitleBar />}
        <div className="app-shell__body">
          <div className="app-shell__content">
            <AppRoutes />
          </div>
        </div>
        <ToastHost />
      </div>
    </HashRouter>
  )
}
