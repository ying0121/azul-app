import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { motion, useReducedMotion } from 'framer-motion'
import { HeaderFavicon3D, HEADER_FAVICON_SIZE } from '@/components/layout/HeaderFavicon3D'
const APP_NAME = 'Daily Huddle'
const TEXT_OFFSET = 48

const TIMING = {
  hiddenPause: 500,
  travel: 1400,
  typeInChar: 42,
  hold: 1400,
  typeOutChar: 28,
} as const

interface HeaderAccentProps {
  clinicName: string
}

function sleep(ms: number, signal: { cancelled: boolean }) {
  return new Promise<void>((resolve) => {
    window.setTimeout(() => {
      if (!signal.cancelled) resolve()
    }, ms)
  })
}

export function HeaderAccent({ clinicName }: HeaderAccentProps) {
  const reduceMotion = useReducedMotion()
  const trackRef = useRef<HTMLDivElement>(null)
  const [trackWidth, setTrackWidth] = useState(0)
  const [faviconX, setFaviconX] = useState(0)
  const [faviconVisible, setFaviconVisible] = useState(false)
  const [typedText, setTypedText] = useState('')
  const [showCursor, setShowCursor] = useState(false)

  const fullText = `${clinicName} | ${APP_NAME}`
  const maxFaviconX = Math.max(0, trackWidth - HEADER_FAVICON_SIZE)

  useLayoutEffect(() => {
    const updateWidth = () => {
      if (trackRef.current) {
        setTrackWidth(trackRef.current.offsetWidth)
      }
    }

    updateWidth()
    window.addEventListener('resize', updateWidth)

    const observer =
      typeof ResizeObserver !== 'undefined'
        ? new ResizeObserver(updateWidth)
        : null

    if (observer && trackRef.current) {
      observer.observe(trackRef.current)
    }

    return () => {
      window.removeEventListener('resize', updateWidth)
      observer?.disconnect()
    }
  }, [])

  useEffect(() => {
    if (reduceMotion) {
      setFaviconX(0)
      setFaviconVisible(true)
      setTypedText(fullText)
      setShowCursor(false)
      return
    }

    if (trackWidth === 0) return

    const signal = { cancelled: false }

    async function runCycle() {
      while (!signal.cancelled) {
        setFaviconX(maxFaviconX)
        setFaviconVisible(false)
        setTypedText('')
        setShowCursor(false)
        await sleep(TIMING.hiddenPause, signal)
        if (signal.cancelled) return

        setFaviconVisible(true)
        setFaviconX(0)
        await sleep(TIMING.travel, signal)
        if (signal.cancelled) return

        setShowCursor(true)
        for (let index = 1; index <= fullText.length; index += 1) {
          if (signal.cancelled) return
          setTypedText(fullText.slice(0, index))
          await sleep(TIMING.typeInChar, signal)
        }

        await sleep(TIMING.hold, signal)
        if (signal.cancelled) return

        for (let index = fullText.length; index >= 0; index -= 1) {
          if (signal.cancelled) return
          setTypedText(fullText.slice(0, index))
          await sleep(TIMING.typeOutChar, signal)
        }

        setShowCursor(false)
        setFaviconX(maxFaviconX)
        await sleep(TIMING.travel, signal)
        if (signal.cancelled) return

        setFaviconVisible(false)
      }
    }

    void runCycle()

    return () => {
      signal.cancelled = true
    }
  }, [clinicName, fullText, maxFaviconX, reduceMotion, trackWidth])

  return (
    <div className="header-accent" ref={trackRef}>
      <div className="header-accent__stage" aria-live="polite">
        <motion.div
          className="header-accent__favicon-wrap"
          animate={{
            x: faviconX,
            opacity: faviconVisible ? 1 : 0,
          }}
          transition={{
            x: { duration: TIMING.travel / 1000, ease: 'easeOut' },
            opacity: { duration: 0.18, ease: 'easeOut' },
          }}
        >
          <HeaderFavicon3D spinning={faviconVisible && !reduceMotion} />
        </motion.div>

        {(typedText || showCursor) && (
          <p
            className="header-accent__text"
            style={{ left: TEXT_OFFSET }}
            aria-label={typedText || undefined}
          >
            <span>{typedText}</span>
            {showCursor && <span className="header-accent__cursor">|</span>}
          </p>
        )}
      </div>
    </div>
  )
}
