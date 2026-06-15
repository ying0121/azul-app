import { BackgroundLayers } from '@/components/auth/BackgroundLayers'
import { ParticleNetwork } from '@/components/auth/ParticleNetwork'

export function AnimatedBackground() {
  return (
    <div className="animated-bg" aria-hidden>
      <BackgroundLayers />
      <ParticleNetwork />
    </div>
  )
}
