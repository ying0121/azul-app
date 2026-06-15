import { motion } from 'framer-motion'
import type { ReactNode } from 'react'
import { Logo } from '@/components/ui/Logo'

interface AuthCardProps {
  title?: string
  subtitle: string
  children: ReactNode
}

export function AuthCard({ title, subtitle, children }: AuthCardProps) {
  return (
    <motion.div
      className="auth-card"
      initial={{ opacity: 0, y: 24, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
    >
      <div className="auth-card__logo">
        <Logo size="lg" showText={false} />
      </div>
      <div className="auth-card__header">
        {title && <h1 className="auth-card__title">{title}</h1>}
        <p className="auth-card__subtitle">{subtitle}</p>
      </div>
      {children}
    </motion.div>
  )
}
