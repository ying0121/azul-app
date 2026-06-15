import { motion } from 'framer-motion'

const orbs = [
  { size: 420, x: '10%', y: '15%', color: 'rgba(59, 130, 246, 0.35)', delay: 0 },
  { size: 320, x: '75%', y: '10%', color: 'rgba(6, 182, 212, 0.3)', delay: 1.2 },
  { size: 280, x: '60%', y: '65%', color: 'rgba(99, 102, 241, 0.28)', delay: 0.6 },
  { size: 200, x: '20%', y: '70%', color: 'rgba(14, 165, 233, 0.25)', delay: 1.8 },
]

const particles = Array.from({ length: 24 }, (_, i) => ({
  id: i,
  left: `${(i * 17 + 7) % 100}%`,
  top: `${(i * 23 + 11) % 100}%`,
  size: 2 + (i % 4),
  duration: 4 + (i % 5),
  delay: (i % 8) * 0.3,
}))

/** Original auth background: gradient, orbs, grid, and subtle floating dots. */
export function BackgroundLayers() {
  return (
    <>
      <div className="animated-bg__gradient" />

      {orbs.map((orb, i) => (
        <motion.div
          key={i}
          className="animated-bg__orb"
          style={{
            width: orb.size,
            height: orb.size,
            left: orb.x,
            top: orb.y,
            background: `radial-gradient(circle, ${orb.color} 0%, transparent 70%)`,
          }}
          animate={{
            x: [0, 30, -20, 0],
            y: [0, -25, 15, 0],
            scale: [1, 1.08, 0.95, 1],
          }}
          transition={{
            duration: 12 + i * 2,
            repeat: Infinity,
            ease: 'easeInOut',
            delay: orb.delay,
          }}
        />
      ))}

      {particles.map((p) => (
        <motion.span
          key={p.id}
          className="animated-bg__particle"
          style={{
            left: p.left,
            top: p.top,
            width: p.size,
            height: p.size,
          }}
          animate={{
            opacity: [0.2, 0.8, 0.2],
            y: [0, -30, 0],
          }}
          transition={{
            duration: p.duration,
            repeat: Infinity,
            ease: 'easeInOut',
            delay: p.delay,
          }}
        />
      ))}

      <motion.div
        className="animated-bg__grid"
        animate={{ backgroundPosition: ['0px 0px', '40px 40px'] }}
        transition={{ duration: 20, repeat: Infinity, ease: 'linear' }}
      />
    </>
  )
}
