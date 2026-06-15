import { useEffect, useRef } from 'react'
import { useThemeStore } from '@/stores/themeStore'

interface Particle {
  x: number
  y: number
  vx: number
  vy: number
  radius: number
}

interface MouseState {
  x: number
  y: number
  active: boolean
}

const CONNECT_DISTANCE = 170
const MOUSE_RADIUS = 130
const MOUSE_FORCE = 1.1
const MAX_SPEED = 2.4

const COLORS = {
  dark: {
    bubble: 'rgba(255, 255, 255, 0.88)',
    line: (opacity: number) => `rgba(255, 255, 255, ${opacity})`,
    lineStrength: 0.38,
  },
  light: {
    bubble: 'rgba(37, 99, 235, 0.72)',
    line: (opacity: number) => `rgba(37, 99, 235, ${opacity})`,
    lineStrength: 0.42,
  },
} as const

function createParticles(width: number, height: number): Particle[] {
  const count = Math.min(200, Math.max(120, Math.floor((width * height) / 6500)))

  return Array.from({ length: count }, () => ({
    x: Math.random() * width,
    y: Math.random() * height,
    vx: (Math.random() - 0.5) * 0.5,
    vy: (Math.random() - 0.5) * 0.5,
    radius: Math.random() * 2.5 + 3.5,
  }))
}

function applyMouseRepulsion(particles: Particle[], mouse: MouseState) {
  if (!mouse.active) return

  for (const particle of particles) {
    const dx = particle.x - mouse.x
    const dy = particle.y - mouse.y
    const distance = Math.hypot(dx, dy)

    if (distance >= MOUSE_RADIUS || distance === 0) continue

    const push = ((MOUSE_RADIUS - distance) / MOUSE_RADIUS) * MOUSE_FORCE
    particle.vx += (dx / distance) * push
    particle.vy += (dy / distance) * push
  }
}

function clampSpeed(particles: Particle[]) {
  for (const particle of particles) {
    const speed = Math.hypot(particle.vx, particle.vy)
    if (speed <= MAX_SPEED) continue

    particle.vx = (particle.vx / speed) * MAX_SPEED
    particle.vy = (particle.vy / speed) * MAX_SPEED
  }
}

/** Transparent overlay: moving bubbles with connecting lines and mouse repulsion. */
export function ParticleNetwork() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const theme = useThemeStore((s) => s.theme)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return

    const ctx = canvas.getContext('2d')
    if (!ctx) return

    let animationId = 0
    let particles: Particle[] = []
    const palette = theme === 'light' ? COLORS.light : COLORS.dark
    const mouse: MouseState = { x: 0, y: 0, active: false }

    const updateMouse = (clientX: number, clientY: number) => {
      const rect = canvas.getBoundingClientRect()
      mouse.x = clientX - rect.left
      mouse.y = clientY - rect.top
      mouse.active = true
    }

    const handleMouseMove = (event: MouseEvent) => {
      updateMouse(event.clientX, event.clientY)
    }

    const handleMouseLeave = () => {
      mouse.active = false
    }

    const resize = () => {
      const dpr = window.devicePixelRatio || 1
      const { width, height } = canvas.getBoundingClientRect()
      canvas.width = Math.floor(width * dpr)
      canvas.height = Math.floor(height * dpr)
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
      particles = createParticles(width, height)
    }

    const tick = () => {
      const width = canvas.clientWidth
      const height = canvas.clientHeight

      ctx.clearRect(0, 0, width, height)

      applyMouseRepulsion(particles, mouse)

      for (const particle of particles) {
        particle.x += particle.vx
        particle.y += particle.vy

        if (particle.x <= particle.radius || particle.x >= width - particle.radius) {
          particle.vx *= -1
        }
        if (particle.y <= particle.radius || particle.y >= height - particle.radius) {
          particle.vy *= -1
        }

        particle.x = Math.max(particle.radius, Math.min(width - particle.radius, particle.x))
        particle.y = Math.max(particle.radius, Math.min(height - particle.radius, particle.y))
      }

      clampSpeed(particles)

      for (let i = 0; i < particles.length; i += 1) {
        for (let j = i + 1; j < particles.length; j += 1) {
          const a = particles[i]
          const b = particles[j]
          const dx = a.x - b.x
          const dy = a.y - b.y
          const distance = Math.hypot(dx, dy)

          if (distance >= CONNECT_DISTANCE) continue

          const opacity = (1 - distance / CONNECT_DISTANCE) * palette.lineStrength
          ctx.strokeStyle = palette.line(opacity)
          ctx.lineWidth = 2
          ctx.beginPath()
          ctx.moveTo(a.x, a.y)
          ctx.lineTo(b.x, b.y)
          ctx.stroke()
        }
      }

      for (const particle of particles) {
        ctx.beginPath()
        ctx.arc(particle.x, particle.y, particle.radius, 0, Math.PI * 2)
        ctx.fillStyle = palette.bubble
        ctx.fill()
      }

      animationId = window.requestAnimationFrame(tick)
    }

    resize()
    tick()

    window.addEventListener('resize', resize)
    window.addEventListener('mousemove', handleMouseMove)
    window.addEventListener('mouseleave', handleMouseLeave)

    return () => {
      window.cancelAnimationFrame(animationId)
      window.removeEventListener('resize', resize)
      window.removeEventListener('mousemove', handleMouseMove)
      window.removeEventListener('mouseleave', handleMouseLeave)
    }
  }, [theme])

  return <canvas ref={canvasRef} className="animated-bg__network" aria-hidden />
}
