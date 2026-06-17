import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { motion } from 'framer-motion'
import { KeyRound } from 'lucide-react'
import { authenticate } from '@/api/auth'
import { AnimatedBackground } from '@/components/auth/AnimatedBackground'
import { AuthCard } from '@/components/auth/AuthCard'
import { ThemeToggle } from '@/components/ui/ThemeToggle'
import { useAlertStore } from '@/stores/alertStore'
import { useAuthStore } from '@/stores/authStore'

export function AuthPage() {
  const navigate = useNavigate()
  const showAlert = useAlertStore((s) => s.show)
  const { setAuthSession, reset } = useAuthStore()
  const [code, setCode] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [statusText, setStatusText] = useState('Continue')
  const [shakeTrigger, setShakeTrigger] = useState(0)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!code.trim()) {
      showAlert('warning', 'Code Required', 'Please enter your access code.')
      return
    }

    setIsSubmitting(true)
    setStatusText('Verifying...')

    try {
      const result = await authenticate({ code: code.trim() })

      if (!result.ok) {
        setShakeTrigger((count) => count + 1)
        showAlert('error', 'Authentication Failed', result.message)
        return
      }

      setAuthSession(result.clinic, result.token)
      navigate('/dashboard', { replace: true })
    } catch (err: unknown) {
      reset()
      showAlert(
        'error',
        'Authentication Failed',
        (err as { friendlyMessage?: string }).friendlyMessage ??
          'Unable to authenticate. Please try again.',
      )
    } finally {
      setIsSubmitting(false)
      setStatusText('Continue')
    }
  }

  return (
    <div className="auth-page">
      <AnimatedBackground />
      <div className="auth-page__toolbar">
        <ThemeToggle />
      </div>
      <AuthCard
        subtitle="Enter your clinic access code to continue"
        shakeTrigger={shakeTrigger}
      >
        <form className="auth-form" onSubmit={(e) => void handleSubmit(e)}>
          <label className="field">
            <span className="field__label">Access Code</span>
            <div className="field__input-wrap">
              <KeyRound size={18} className="field__icon" />
              <input
                type="password"
                autoComplete="off"
                placeholder="Enter access code"
                value={code}
                onChange={(e) => setCode(e.target.value)}
                disabled={isSubmitting}
                autoFocus
              />
            </div>
          </label>

          <motion.button
            type="submit"
            className="btn btn--primary btn--full"
            disabled={isSubmitting}
            whileHover={{ scale: 1.01 }}
            whileTap={{ scale: 0.98 }}
          >
            {isSubmitting ? statusText : 'Continue'}
          </motion.button>
        </form>

        {import.meta.env.VITE_USE_MOCK === 'true' && (
          <p className="auth-hint">Demo code: roswell123</p>
        )}
      </AuthCard>
    </div>
  )
}
