import { motion } from 'framer-motion'
import { LogOut } from 'lucide-react'
import { Logo } from '@/components/ui/Logo'
import { Avatar } from '@/components/ui/Avatar'
import { ThemeToggle } from '@/components/ui/ThemeToggle'
import { logout } from '@/api/auth'
import { useAuthStore } from '@/stores/authStore'
import { useStatusColorStore } from '@/stores/statusColorStore'
import { getClinicDisplayName } from '@/types/auth'
import { useNavigate } from 'react-router-dom'

export function Header() {
  const { clinic, reset } = useAuthStore()
  const navigate = useNavigate()
  const clinicName = getClinicDisplayName(clinic)

  const handleLogout = async () => {
    await logout()
    reset()
    useStatusColorStore.getState().reset()
    navigate('/auth', { replace: true })
  }

  return (
    <motion.header
      className="header"
      initial={{ opacity: 0, y: -12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4 }}
    >
      <Logo size="sm" />

      <div className="header__message">
        <span className="header__greeting">Welcome back, {clinicName}</span>
        <span className="header__hint">
          Review patient quality measures and medication adherence
        </span>
      </div>

      <div className="header__actions">
        {clinic && <Avatar name={clinicName} size="md" />}
        <ThemeToggle />
        <button
          className="header__logout btn btn--ghost"
          onClick={() => void handleLogout()}
          title="Sign out"
        >
          <LogOut size={18} />
          <span>Sign out</span>
        </button>
      </div>
    </motion.header>
  )
}
