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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
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
    token: context.huddleToken,
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
  insId: string
  qpId: string
  clinicName: string
  insuranceName: string
  qualityProgramName: string
  statusMap: HedisStatusMap
}): Promise<DailyEmailPreviewData> {
  const { rows, token } = await fetchTodayVisitPatients({
    clinic_id: options.clinicId,
    ins_id: options.insId,
    qp_id: options.qpId,
  })

  const reportDate = getTodayReportDateLabel()
  const context: DailyVisitEmailContext = {
    clinicName: options.clinicName,
    reportDate,
    insuranceName: options.insuranceName,
    qualityProgramName: options.qualityProgramName,
    huddleToken: token,
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
