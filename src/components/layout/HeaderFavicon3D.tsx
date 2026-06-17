import { useEffect, useRef } from 'react'
import clsx from 'clsx'
import * as THREE from 'three'

const FAVICON_SRC = '/favicon.ico'
export const HEADER_FAVICON_SIZE = 40
const SPIN_SPEED = 0.028

interface HeaderFavicon3DProps {
  spinning: boolean
  className?: string
}

function loadFaviconTexture(): Promise<THREE.Texture> {
  return new Promise((resolve, reject) => {
    const image = new Image()
    image.decoding = 'async'

    image.onload = () => {
      const texture = new THREE.Texture(image)
      texture.colorSpace = THREE.SRGBColorSpace
      texture.needsUpdate = true
      resolve(texture)
    }

    image.onerror = () => reject(new Error('Unable to load favicon texture.'))
    image.src = FAVICON_SRC
  })
}

export function HeaderFavicon3D({ spinning, className }: HeaderFavicon3DProps) {
  const hostRef = useRef<HTMLDivElement>(null)
  const spinningRef = useRef(spinning)

  spinningRef.current = spinning

  useEffect(() => {
    const host = hostRef.current
    if (!host) return

    const size = HEADER_FAVICON_SIZE
    let disposed = false
    let frameId = 0

    const scene = new THREE.Scene()
    const camera = new THREE.PerspectiveCamera(28, 1, 0.1, 20)
    camera.position.set(0, 0, 3.1)

    const renderer = new THREE.WebGLRenderer({
      alpha: true,
      antialias: true,
      powerPreference: 'low-power',
    })
    renderer.setSize(size, size)
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    renderer.setClearColor(0x000000, 0)
    host.appendChild(renderer.domElement)

    const resources: Array<THREE.Material | THREE.Texture | THREE.BufferGeometry> = []
    let mesh: THREE.Mesh | null = null

    void loadFaviconTexture()
      .then((texture) => {
        if (disposed) {
          texture.dispose()
          return
        }

        resources.push(texture)

        const material = new THREE.MeshBasicMaterial({
          map: texture,
          transparent: true,
          alphaTest: 0.08,
          side: THREE.DoubleSide,
        })

        resources.push(material)

        const geometry = new THREE.PlaneGeometry(1.65, 1.65)
        resources.push(geometry)

        mesh = new THREE.Mesh(geometry, material)
        scene.add(mesh)

        const render = () => {
          if (disposed) return

          frameId = window.requestAnimationFrame(render)

          if (mesh && spinningRef.current) {
            mesh.rotation.y += SPIN_SPEED
          }

          renderer.render(scene, camera)
        }

        render()
      })
      .catch(() => {
        if (!disposed) {
          host.classList.add('header-favicon-3d--fallback')
        }
      })

    return () => {
      disposed = true
      window.cancelAnimationFrame(frameId)

      if (mesh) {
        scene.remove(mesh)
      }

      resources.forEach((resource) => resource.dispose())
      renderer.dispose()

      if (renderer.domElement.parentElement === host) {
        host.removeChild(renderer.domElement)
      }
    }
  }, [])

  return (
    <div ref={hostRef} className={clsx('header-favicon-3d', className)} aria-hidden>
      <img src={FAVICON_SRC} alt="" className="header-favicon-3d__fallback" />
    </div>
  )
}
