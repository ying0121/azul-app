import { apiClient, USE_MOCK } from './client'
import { fetchPatients, type PatientsFetchResult } from './patients'
import {
  buildDailyVisitEmailHtml,
  buildDailyVisitEmailSubject,
  buildDailyVisitEmailText,
  getTodayReportDateLabel,
  type DailyVisitEmailContext,
} from '@/lib/dailyVisitEmailTemplate'
import { getClinicTodayDateString } from '@/lib/clinicDate'
import { getCurrentYear } from '@/types/filters'
import type { PatientFilters } from '@/types/filters'
import type { HedisStatusMap } from '@/types/statusColor'

export interface SendDailyEmailRequest {
  clinic_id: string | number
  ins_id: string
  qp_id: string
  token: string
  subject: string
  html: string
  text: string
  report_date: string
}

interface SendDailyEmailApiResponse {
  status: string
  message?: string
}

interface TokenizationApiResponse {
  status: string
  tokenization?: number
  message?: string
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function readFiniteNumber(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value)) return value

  if (typeof value === 'string') {
    const trimmed = value.trim()
    if (!trimmed) return null
    const parsed = Number(trimmed)
    if (Number.isFinite(parsed)) return parsed
  }

  return null
}

function parseTokenizationResponse(data: unknown): number | null {
  if (!isRecord(data) || data.status !== 'success') return null

  const direct = readFiniteNumber(data.tokenization ?? data.Tokenization)
  if (direct != null) return direct

  const nested = data.data ?? data.Data
  if (isRecord(nested)) {
    return readFiniteNumber(nested.tokenization ?? nested.Tokenization)
  }

  return null
}

export async function fetchTokenization(clinicId: string | number): Promise<number> {
  if (USE_MOCK) {
    await delay(200)
    return 42
  }

  const { data } = await apiClient.post<TokenizationApiResponse>('/daily-huddle/tokenization', {
    clinic_id: clinicId,
  })

  const tokenization = parseTokenizationResponse(data)
  if (tokenization == null) {
    throw new Error(
      isRecord(data) && typeof data.message === 'string' && data.message.trim()
        ? data.message.trim()
        : 'Unable to fetch clinic tokenization.',
    )
  }

  return tokenization
}

export async function fetchTodayVisitPatients(
  filters: Pick<PatientFilters, 'clinic_id' | 'ins_id' | 'qp_id'>,
): Promise<PatientsFetchResult> {
  const today = getClinicTodayDateString()

  return fetchPatients({
    ...filters,
    cyear: getCurrentYear(),
    filter: '',
    appt_start: today,
    appt_end: today,
  })
}

export function buildDailyEmailPayload(
  context: DailyVisitEmailContext,
  clinicId: string | number,
  insId: string,
  qpId: string,
): SendDailyEmailRequest {
  const reportDate = getTodayReportDateLabel()

  return {
    clinic_id: clinicId,
    ins_id: insId,
    qp_id: qpId,
    token: String(context.tokenization),
    subject: buildDailyVisitEmailSubject({ ...context, reportDate }),
    html: buildDailyVisitEmailHtml({ ...context, reportDate }),
    text: buildDailyVisitEmailText({ ...context, reportDate }),
    report_date: getClinicTodayDateString(),
  }
}

export interface DailyEmailPreviewSummary {
  clinicName: string
  reportDate: string
  insuranceName: string
  qualityProgramName: string
  total: number
  hedisCount: number
  medAdhCount: number
}

export interface DailyEmailPreviewData {
  payload: SendDailyEmailRequest
  summary: DailyEmailPreviewSummary
}

export async function prepareDailyVisitEmail(options: {
  clinicId: string | number
  clinicAcronym: string
  insId: string
  qpId: string
  clinicName: string
  insuranceName: string
  qualityProgramName: string
  statusMap: HedisStatusMap
}): Promise<DailyEmailPreviewData> {
  const [{ rows }, tokenization] = await Promise.all([
    fetchTodayVisitPatients({
      clinic_id: options.clinicId,
      ins_id: options.insId,
      qp_id: options.qpId,
    }),
    fetchTokenization(options.clinicId),
  ])

  const reportDate = getTodayReportDateLabel()
  const context: DailyVisitEmailContext = {
    clinicName: options.clinicName,
    clinicAcronym: options.clinicAcronym,
    reportDate,
    insuranceName: options.insuranceName,
    qualityProgramName: options.qualityProgramName,
    tokenization,
    rows,
    statusMap: options.statusMap,
  }

  const payload = buildDailyEmailPayload(
    context,
    options.clinicId,
    options.insId,
    options.qpId,
  )

  const hedisCount = rows.filter((row) => row.source === 'hedis').length
  const medAdhCount = rows.filter((row) => row.source === 'med_adh').length

  return {
    payload,
    summary: {
      clinicName: options.clinicName,
      reportDate,
      insuranceName: options.insuranceName,
      qualityProgramName: options.qualityProgramName,
      total: rows.length,
      hedisCount,
      medAdhCount,
    },
  }
}

export async function sendDailyEmail(payload: SendDailyEmailRequest): Promise<string> {
  if (USE_MOCK) {
    await delay(800)
    return `Daily visit email prepared for ${payload.report_date} (${payload.subject}).`
  }

  const { data } = await apiClient.post<SendDailyEmailApiResponse>(
    '/daily-huddle/daily-email',
    payload,
  )

  if (!isRecord(data) || data.status !== 'success') {
    throw new Error(
      isRecord(data) && typeof data.message === 'string' && data.message.trim()
        ? data.message.trim()
        : 'Unable to send daily email.',
    )
  }

  if (typeof data.message === 'string' && data.message.trim()) {
    return data.message.trim()
  }

  return 'Daily visit email sent successfully.'
}

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}
