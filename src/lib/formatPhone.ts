import { isNullishValue } from '@/lib/formatDate'

const US_COUNTRY_PREFIX = /^\+?1(?:[\s().-]+|\s*)(?=\d)/

export function formatPhoneDisplay(value: string | number | null | undefined): string {
  if (isNullishValue(value)) return ''

  const trimmed = String(value).trim()
  if (!trimmed) return ''

  const withoutPrefix = trimmed.replace(US_COUNTRY_PREFIX, '').replace(/^1([\s().-]+)(?=\d)/, '')
  if (withoutPrefix !== trimmed) {
    return withoutPrefix.trim()
  }

  const digits = trimmed.replace(/\D/g, '')
  if (digits.length === 11 && digits.startsWith('1')) {
    return digits.slice(1)
  }

  return trimmed
}

export function formatPhoneTelHref(value: string | number | null | undefined): string {
  const digits = formatPhoneDisplay(value).replace(/\D/g, '')
  return digits ? `tel:${digits}` : ''
}

export const PHONE_DETAIL_KEYS = new Set(['pt_phone', 'pt_other_phone', 'med1_pham_phone'])

export function formatPhoneDetailValue(value: string | number | null | undefined): string {
  const formatted = formatPhoneDisplay(value)
  return formatted || '—'
}
