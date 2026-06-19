import clsx from 'clsx'

interface LogoProps {
  size?: 'sm' | 'md' | 'lg'
  showText?: boolean
  className?: string
}

const LOGO_SRC = '/logo.png'
const FAVICON_SRC = '/favicon.ico'

const sizes = {
  sm: { mark: 32, fullWidth: 140, text: '0.95rem' },
  md: { mark: 40, fullWidth: 180, text: '1.1rem' },
  lg: { mark: 48, fullWidth: 260, text: '1.25rem' },
}

export function Logo({ size = 'md', showText = true, className }: LogoProps) {
  const config = sizes[size]
  const useFullLogo = size === 'lg' || !showText

  return (
    <div className={clsx('logo', 'logo--enter', `logo--${size}`, className)}>
      <img
        src={useFullLogo ? LOGO_SRC : FAVICON_SRC}
        alt="Precision Quality"
        className={clsx('logo__image', useFullLogo && 'logo__image--full')}
        style={
          useFullLogo
            ? { width: config.fullWidth, maxWidth: '100%' }
            : { width: config.mark, height: config.mark }
        }
      />
      {showText && size !== 'lg' && (
        <div className="logo__text" style={{ fontSize: config.text }}>
          <span className="logo__title">Daily Huddle</span>
          <span className="logo__subtitle">Quality Care Dashboard</span>
        </div>
      )}
    </div>
  )
}
