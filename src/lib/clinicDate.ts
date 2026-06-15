export const CLINIC_TIMEZONE =
  import.meta.env.VITE_CLINIC_TIMEZONE?.trim() || 'America/New_York'

type DateParts = { year: number; month: number; day: number }

function getZonedDateParts(date: Date, timeZone: string): DateParts {
  const formatter = new Intl.DateTimeFormat('en-US', {
    timeZone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  })

  const parts = formatter.formatToParts(date)
  const read = (type: Intl.DateTimeFormatPart['type']) =>
    Number(parts.find((part) => part.type === type)?.value ?? 0)

  return { year: read('year'), month: read('month'), day: read('day') }
}

function toDateString(parts: DateParts): string {
  return `${parts.year}-${String(parts.month).padStart(2, '0')}-${String(parts.day).padStart(2, '0')}`
}

export function getClinicTodayDateString(date: Date = new Date()): string {
  return toDateString(getZonedDateParts(date, CLINIC_TIMEZONE))
}

export function getClinicYear(date: Date = new Date()): number {
  return getZonedDateParts(date, CLINIC_TIMEZONE).year
}

/** Add calendar days to a YYYY-MM-DD string. */
export function addCalendarDays(dateStr: string, days: number): string {
  const [year, month, day] = dateStr.split('-').map(Number)
  const utc = new Date(Date.UTC(year, month - 1, day))
  utc.setUTCDate(utc.getUTCDate() + days)
  return `${utc.getUTCFullYear()}-${String(utc.getUTCMonth() + 1).padStart(2, '0')}-${String(utc.getUTCDate()).padStart(2, '0')}`
}
