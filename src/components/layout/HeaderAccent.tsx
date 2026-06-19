import { useReducedMotion } from 'framer-motion'

const APP_NAME = 'Daily Team Huddle'

interface HeaderAccentProps {
  clinicName: string
}

export function HeaderAccent({ clinicName }: HeaderAccentProps) {
  const reduceMotion = useReducedMotion()
  const fullText = `${clinicName} | ${APP_NAME}`

  return (
    <div className="header-accent">
      <p
        className="header-accent__text"
        aria-label={fullText}
        data-animated={!reduceMotion ? 'true' : undefined}
      >
        {fullText}
      </p>
    </div>
  )
}
