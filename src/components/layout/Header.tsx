import { motion } from 'framer-motion'
import { LogOut } from 'lucide-react'
import { HeaderAccent } from '@/components/layout/HeaderAccent'
import { Avatar } from '@/components/ui/Avatar'
import { ThemeToggle } from '@/components/ui/ThemeToggle'
import { logout } from '@/api/auth'
import { useAuthStore } from '@/stores/authStore'
import { useStatusColorStore } from '@/stores/statusColorStore'
import { getClinicAcronym, getClinicDisplayName } from '@/types/auth'
import { useNavigate } from 'react-router-dom'

export function Header() {
  const { clinic, reset } = useAuthStore()
  const navigate = useNavigate()
  const clinicName = getClinicDisplayName(clinic)
  const clinicAcronym = getClinicAcronym(clinic)

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
      <HeaderAccent clinicName={clinicName} />

      <div className="header__actions">
        {clinic && (
          <Avatar acronym={clinicAcronym} name={clinicName} size="lg" />
        )}
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
