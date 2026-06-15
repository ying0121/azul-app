import { apiClient, USE_MOCK } from './client'
import { mockStatusColors } from './mock'
import type {
  HedisStatusMap,
  StatusColorItem,
  StatusColorsByType,
} from '@/types/statusColor'
import {
  groupStatusColorsByType,
} from '@/types/statusColor'

interface StatusColorApiResponse {
  status: string
  data?: unknown[]
  message?: string
}

export interface StatusColorData {
  map: HedisStatusMap
  byType: StatusColorsByType
  items: StatusColorItem[]
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function readString(data: Record<string, unknown>, ...keys: string[]): string {
  for (const key of keys) {
    const value = data[key]
    if (typeof value === 'string') return value
    if (typeof value === 'number' && !Number.isNaN(value)) return String(value)
  }
  return ''
}

function parseStatusType(item: Record<string, unknown>): StatusColorItem['type'] {
  const raw = readString(
    item,
    'status_type',
    'StatusType',
    'type',
    'Type',
    'category',
    'Category',
    'status_category',
    'StatusCategory',
  ).toLowerCase()

  if (raw.includes('patient') || raw === 'p' || raw === 'pt') return 'patient'
  if (raw.includes('eligib') || raw === 'e') return 'eligibility'
  if (raw.includes('report') || raw === 'r') return 'measure_report'
  return 'measure'
}

function parseStatusColorItem(item: unknown): StatusColorItem | null {
  if (!isRecord(item)) return null

  const idRaw = item.id ?? item.Id
  const id = typeof idRaw === 'number' ? idRaw : parseInt(String(idRaw ?? ''), 10)
  if (Number.isNaN(id)) return null

  const display = readString(
    item,
    'measure_status_display',
    'MeasureStatusDisplay',
    'display',
    'Display',
    'name',
    'Name',
  )
  const status = readString(
    item,
    'status_category_display',
    'StatusCategoryDisplay',
    'status',
    'Status',
  )
  const description = readString(
    item,
    'status_description',
    'StatusDescription',
    'description',
    'Description',
    'measure_status_description',
    'MeasureStatusDescription',
  )

  return {
    id,
    type: parseStatusType(item),
    display: display || status,
    status,
    description: description || status,
    text_color: readString(item, 'text_color', 'textColor', 'TextColor'),
    bg_color: readString(item, 'bg_color', 'bgColor', 'BgColor'),
  }
}

export function buildHedisStatusMap(items: StatusColorItem[]): HedisStatusMap {
  const map: HedisStatusMap = {}

  items.forEach((item) => {
    map[item.id] = {
      display: item.display,
      status: item.status,
      description: item.description,
      text_color: item.text_color,
      bg_color: item.bg_color,
    }
  })

  return map
}

export function buildStatusColorData(items: unknown[]): StatusColorData {
  const parsed = items
    .map(parseStatusColorItem)
    .filter((item): item is StatusColorItem => item != null)

  return {
    items: parsed,
    map: buildHedisStatusMap(parsed),
    byType: groupStatusColorsByType(parsed),
  }
}

export async function fetchStatusColors(): Promise<StatusColorData> {
  if (USE_MOCK) {
    await delay(200)
    return buildStatusColorData(mockStatusColors)
  }

  const { data } = await apiClient.get<StatusColorApiResponse>('/daily-huddle/status-color')

  if (!isRecord(data) || data.status !== 'success' || !Array.isArray(data.data)) {
    throw new Error(
      isRecord(data) && typeof data.message === 'string'
        ? data.message
        : 'Unable to load status colors.',
    )
  }

  return buildStatusColorData(data.data)
}

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}
