function toDateString(date: Date): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function getClinicTodayDateString(date: Date = new Date()): string {
  return toDateString(date)
}

export function getClinicYear(date: Date = new Date()): number {
  return date.getFullYear()
}

/** Add calendar days to a YYYY-MM-DD string. */
export function addCalendarDays(dateStr: string, days: number): string {
  const [year, month, day] = dateStr.split('-').map(Number)
  const date = new Date(year, month - 1, day)
  date.setDate(date.getDate() + days)
  return toDateString(date)
}
