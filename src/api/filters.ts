import { apiClient, USE_MOCK } from './client'
import { mockInsurance, mockQualityPrograms } from './mock'
import type { InsuranceOption, QualityProgramOption } from '@/types/filters'

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function readString(data: Record<string, unknown>, ...keys: string[]): string {
  for (const key of keys) {
    const value = data[key]
    if (typeof value === 'string' && value.trim()) return value.trim()
    if (typeof value === 'number') return String(value)
  }
  return ''
}

function unwrapList(response: unknown): unknown[] {
  if (Array.isArray(response)) return response
  if (!isRecord(response)) return []

  const nested = response.data ?? response.Data ?? response.items ?? response.Items
  if (Array.isArray(nested)) return nested

  return []
}

function parseInsuranceItem(item: unknown): InsuranceOption | null {
  if (!isRecord(item)) return null

  const ins_id = readString(item, 'ins_id', 'insId', 'InsId', 'id', 'Id')
  const ins_name = readString(item, 'ins_name', 'insName', 'InsName', 'name', 'Name')

  if (!ins_id || !ins_name) return null

  return { ins_id, ins_name }
}

function parseQualityProgramItem(item: unknown): QualityProgramOption | null {
  if (!isRecord(item)) return null

  const qp_id = readString(item, 'qp_id', 'qpId', 'QpId', 'id', 'Id')
  const qp_name = readString(item, 'qp_name', 'qpName', 'QpName', 'name', 'Name')

  if (!qp_id || !qp_name) return null

  return { qp_id, qp_name }
}

export async function fetchInsuranceList(
  clinicId: string | number,
): Promise<InsuranceOption[]> {
  if (USE_MOCK) {
    await delay(250)
    return mockInsurance
  }

  const { data } = await apiClient.post<unknown>('/daily-huddle/insurance', {
    clinic_id: clinicId,
  })

  return unwrapList(data)
    .map(parseInsuranceItem)
    .filter((item): item is InsuranceOption => item != null)
}

export async function fetchQualityPrograms(
  clinicId: string | number,
  insId: string,
): Promise<QualityProgramOption[]> {
  if (USE_MOCK) {
    await delay(250)
    return mockQualityPrograms[insId] ?? []
  }

  const { data } = await apiClient.post<unknown>('/daily-huddle/quality-program', {
    clinic_id: clinicId,
    ins_id: insId,
  })

  return unwrapList(data)
    .map(parseQualityProgramItem)
    .filter((item): item is QualityProgramOption => item != null)
}

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}
