const ISO_DATE_PREFIX = /^(\d{4})-(\d{2})-(\d{2})/
const US_DATE = /^(\d{1,2})\/(\d{1,2})\/(\d{4})$/

export function isNullishValue(value: string | number | null | undefined): boolean {
  if (value == null) return true

  const raw = String(value).trim().toLowerCase()
  return raw === '' || raw === 'null' || raw === 'undefined'
}

export function normalizeDisplayValue(value: string | number | null | undefined): string {
  if (isNullishValue(value)) return ''
  return String(value).trim()
}

export function formatUsDate(value: string | number | null | undefined): string {
  if (isNullishValue(value)) return ''

  const raw = String(value).trim()

  const isoMatch = raw.match(ISO_DATE_PREFIX)
  if (isoMatch) {
    const [, year, month, day] = isoMatch
    return `${month}/${day}/${year}`
  }

  if (US_DATE.test(raw)) return raw

  const parsed = new Date(raw)
  if (!Number.isNaN(parsed.getTime())) {
    return parsed.toLocaleDateString('en-US', {
      month: '2-digit',
      day: '2-digit',
      year: 'numeric',
    })
  }

  return raw
}

export const DATE_DETAIL_KEYS = new Set([
  'dos',
  'pt_dob',
  'appt_date',
  'admit_date',
  'event_date',
  'discharge_date',
  'med1_dos',
  'med1_refill_date',
  'rx_dates_given',
])

export function formatDetailValue(
  key: string,
  value: string | number | null | undefined,
): string {
  if (isNullishValue(value)) return '—'
  if (DATE_DETAIL_KEYS.has(key)) return formatUsDate(value) || '—'
  return normalizeDisplayValue(value) || '—'
}
