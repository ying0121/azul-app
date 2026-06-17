import { motion } from 'framer-motion'
import { useEffect, useState } from 'react'
import clsx from 'clsx'
import type { ReactNode } from 'react'
import { Logo } from '@/components/ui/Logo'

interface AuthCardProps {
  title?: string
  subtitle: string
  children: ReactNode
  shakeTrigger?: number
}

const SHAKE_DURATION_MS = 520

export function AuthCard({ title, subtitle, children, shakeTrigger = 0 }: AuthCardProps) {
  const [isShaking, setIsShaking] = useState(false)

  useEffect(() => {
    if (shakeTrigger === 0) return

    setIsShaking(false)
    const startFrame = window.requestAnimationFrame(() => {
      setIsShaking(true)
    })
    const timer = window.setTimeout(() => setIsShaking(false), SHAKE_DURATION_MS)

    return () => {
      window.cancelAnimationFrame(startFrame)
      window.clearTimeout(timer)
    }
  }, [shakeTrigger])

  return (
    <motion.div
      className="auth-card-shell"
      initial={{ opacity: 0, y: 24, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
    >
      <div className={clsx('auth-card', isShaking && 'auth-card--shake')}>
        <div className="auth-card__logo">
          <Logo size="lg" showText={false} />
        </div>
        <div className="auth-card__header">
          {title && <h1 className="auth-card__title">{title}</h1>}
          <p className="auth-card__subtitle">{subtitle}</p>
        </div>
        {children}
      </div>
    </motion.div>
  )
}
