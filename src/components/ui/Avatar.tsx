import clsx from 'clsx'

interface AvatarProps {
  name: string
  imageUrl?: string
  size?: 'sm' | 'md' | 'lg'
}

function getInitials(name: string) {
  return name
    .split(' ')
    .map((part) => part[0])
    .join('')
    .slice(0, 2)
    .toUpperCase()
}

export function Avatar({ name, imageUrl, size = 'md' }: AvatarProps) {
  return (
    <div className={clsx('avatar', `avatar--${size}`)} title={name}>
      {imageUrl ? (
        <img src={imageUrl} alt={name} className="avatar__img" />
      ) : (
        <span className="avatar__initials">{getInitials(name)}</span>
      )}
    </div>
  )
}
