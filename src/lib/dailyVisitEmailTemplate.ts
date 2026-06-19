import { getClinicTodayDateString } from '@/lib/clinicDate'
import { formatUsDate } from '@/lib/formatDate'
import { getRowApptDate, getRowValue1, getRowValue2 } from '@/lib/patientRowValues'
import type { PatientRow } from '@/types/patient'
import type { HedisStatusMap } from '@/types/statusColor'
import { resolveRowStatusStyle } from '@/types/statusColor'

export interface DailyVisitEmailContext {
  clinicName: string
  reportDate: string
  insuranceName: string
  qualityProgramName: string
  huddleToken: string
  rows: PatientRow[]
  statusMap: HedisStatusMap
}

function escapeHtml(value: string | number | null | undefined): string {
  return String(value ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function getApptDate(row: PatientRow): string {
  return getRowApptDate(row)
}

function getDoctorName(row: PatientRow): string {
  return `${row.pcp_fname} ${row.pcp_lname}`.trim()
}

function getTokenMid(huddleToken: string, row: PatientRow): string {
  const mid = row.pt_subno?.trim() ?? ''
  if (!huddleToken && !mid) return '—'
  return `${huddleToken}${mid}`
}

function buildSummary(rows: PatientRow[]) {
  const hedisCount = rows.filter((row) => row.source === 'hedis').length
  const medAdhCount = rows.filter((row) => row.source === 'med_adh').length
  return { total: rows.length, hedisCount, medAdhCount }
}

function buildTableRows(
  rows: PatientRow[],
  huddleToken: string,
  statusMap: HedisStatusMap,
): string {
  if (rows.length === 0) {
    return `
      <tr>
        <td colspan="7" style="padding:28px 16px;text-align:center;color:#64748b;font-size:14px;">
          No visits scheduled for today.
        </td>
      </tr>
    `
  }

  return rows
    .map((row, index) => {
      const rowStyle = resolveRowStatusStyle(statusMap, row.details)
      const backgroundColor = rowStyle.backgroundColor || (index % 2 === 0 ? '#ffffff' : '#f8fafc')
      const textColor = rowStyle.color || '#0f172a'

      return `
        <tr style="background-color:${backgroundColor};color:${textColor};">
          <td style="padding:12px 14px;border-bottom:1px solid #e2e8f0;font-size:13px;font-weight:600;">${escapeHtml(getTokenMid(huddleToken, row))}</td>
          <td style="padding:12px 14px;border-bottom:1px solid #e2e8f0;font-size:13px;">${escapeHtml(row.ins_name || row.ins_id || '—')}</td>
          <td style="padding:12px 14px;border-bottom:1px solid #e2e8f0;font-size:13px;">${escapeHtml(row.measure || '—')}</td>
          <td style="padding:12px 14px;border-bottom:1px solid #e2e8f0;font-size:13px;">${escapeHtml(getDoctorName(row) || '—')}</td>
          <td style="padding:12px 14px;border-bottom:1px solid #e2e8f0;font-size:13px;">${escapeHtml(getApptDate(row))}</td>
          <td style="padding:12px 14px;border-bottom:1px solid #e2e8f0;font-size:13px;">${escapeHtml(getRowValue1(row))}</td>
          <td style="padding:12px 14px;border-bottom:1px solid #e2e8f0;font-size:13px;">${escapeHtml(getRowValue2(row))}</td>
        </tr>
      `
    })
    .join('')
}

export function buildDailyVisitEmailSubject(context: DailyVisitEmailContext): string {
  return `Daily Team Huddle Visit Report — ${context.clinicName} — ${context.reportDate}`
}

export function buildDailyVisitEmailHtml(context: DailyVisitEmailContext): string {
  const {
    clinicName,
    reportDate,
    insuranceName,
    qualityProgramName,
    huddleToken,
    rows,
    statusMap,
  } = context
  const summary = buildSummary(rows)
  const tableRows = buildTableRows(rows, huddleToken, statusMap)

  return `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Daily Team Huddle Visit Report</title>
  </head>
  <body style="margin:0;padding:0;background-color:#eef2f7;font-family:Segoe UI, Arial, sans-serif;color:#0f172a;">
    <table role="presentation" width="100%" cellspacing="0" cellpadding="0" border="0" style="background-color:#eef2f7;padding:24px 12px;">
      <tr>
        <td align="center">
          <table role="presentation" width="100%" cellspacing="0" cellpadding="0" border="0" style="max-width:1080px;background-color:#ffffff;border-radius:18px;overflow:hidden;border:1px solid #dbe3f0;box-shadow:0 12px 30px rgba(15,23,42,0.08);">
            <tr>
              <td style="padding:28px 32px;background:linear-gradient(135deg,#1d4ed8 0%,#2563eb 55%,#06b6d4 100%);color:#ffffff;">
                <table role="presentation" width="100%" cellspacing="0" cellpadding="0" border="0">
                  <tr>
                    <td>
                      <div style="font-size:12px;font-weight:700;letter-spacing:0.12em;text-transform:uppercase;opacity:0.88;margin-bottom:8px;">
                        Daily Team Huddle
                      </div>
                      <div style="font-size:28px;line-height:1.2;font-weight:700;margin-bottom:8px;">
                        Today's Visit Report
                      </div>
                      <div style="font-size:15px;line-height:1.5;opacity:0.95;">
                        ${escapeHtml(clinicName)} &bull; ${escapeHtml(reportDate)}
                      </div>
                    </td>
                    <td align="right" valign="top" style="font-size:13px;line-height:1.6;opacity:0.95;">
                      <div><strong>Insurance:</strong> ${escapeHtml(insuranceName || '—')}</div>
                      <div><strong>Quality Program:</strong> ${escapeHtml(qualityProgramName || '—')}</div>
                    </td>
                  </tr>
                </table>
              </td>
            </tr>

            <tr>
              <td style="padding:24px 32px 8px;">
                <table role="presentation" width="100%" cellspacing="0" cellpadding="0" border="0">
                  <tr>
                    <td width="33%" style="padding:0 8px 16px 0;">
                      <div style="background:#f8fafc;border:1px solid #e2e8f0;border-radius:14px;padding:18px 20px;">
                        <div style="font-size:12px;font-weight:700;color:#64748b;text-transform:uppercase;letter-spacing:0.06em;">Total Visits</div>
                        <div style="font-size:30px;font-weight:700;color:#1d4ed8;margin-top:8px;">${summary.total}</div>
                      </div>
                    </td>
                    <td width="33%" style="padding:0 8px 16px;">
                      <div style="background:#eff6ff;border:1px solid #bfdbfe;border-radius:14px;padding:18px 20px;">
                        <div style="font-size:12px;font-weight:700;color:#64748b;text-transform:uppercase;letter-spacing:0.06em;">HEDIS</div>
                        <div style="font-size:30px;font-weight:700;color:#2563eb;margin-top:8px;">${summary.hedisCount}</div>
                      </div>
                    </td>
                    <td width="33%" style="padding:0 0 16px 8px;">
                      <div style="background:#ecfeff;border:1px solid #a5f3fc;border-radius:14px;padding:18px 20px;">
                        <div style="font-size:12px;font-weight:700;color:#64748b;text-transform:uppercase;letter-spacing:0.06em;">Med Adh</div>
                        <div style="font-size:30px;font-weight:700;color:#0891b2;margin-top:8px;">${summary.medAdhCount}</div>
                      </div>
                    </td>
                  </tr>
                </table>
              </td>
            </tr>

            <tr>
              <td style="padding:8px 32px 28px;">
                <div style="font-size:18px;font-weight:700;color:#0f172a;margin-bottom:14px;">
                  Daily Visit Table
                </div>
                <div style="overflow-x:auto;border:1px solid #e2e8f0;border-radius:14px;">
                  <table role="presentation" width="100%" cellspacing="0" cellpadding="0" border="0" style="border-collapse:collapse;min-width:640px;">
                    <thead>
                      <tr style="background-color:#f1f5f9;">
                        <th align="left" style="padding:12px 14px;font-size:11px;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.05em;border-bottom:1px solid #dbe3f0;">Patient</th>
                        <th align="left" style="padding:12px 14px;font-size:11px;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.05em;border-bottom:1px solid #dbe3f0;">Insurance</th>
                        <th align="left" style="padding:12px 14px;font-size:11px;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.05em;border-bottom:1px solid #dbe3f0;">Measure</th>
                        <th align="left" style="padding:12px 14px;font-size:11px;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.05em;border-bottom:1px solid #dbe3f0;">Doctor</th>
                        <th align="left" style="padding:12px 14px;font-size:11px;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.05em;border-bottom:1px solid #dbe3f0;">Appt Date</th>
                        <th align="left" style="padding:12px 14px;font-size:11px;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.05em;border-bottom:1px solid #dbe3f0;">Value 1</th>
                        <th align="left" style="padding:12px 14px;font-size:11px;font-weight:700;color:#475569;text-transform:uppercase;letter-spacing:0.05em;border-bottom:1px solid #dbe3f0;">Value 2</th>
                      </tr>
                    </thead>
                    <tbody>
                      ${tableRows}
                    </tbody>
                  </table>
                </div>
              </td>
            </tr>

            <tr>
              <td style="padding:0 32px 28px;">
                <div style="background:#f8fafc;border:1px solid #e2e8f0;border-radius:12px;padding:16px 18px;font-size:12px;line-height:1.6;color:#64748b;">
                  This report was generated automatically from Conector Health.
                  Patient identifiers don't use real personal information. They are generated by the system.
                  Row colors reflect configured measure and patient status colors for quick review.
                </div>
              </td>
            </tr>

            <tr>
              <td style="padding:18px 32px 28px;background:#f8fafc;border-top:1px solid #e2e8f0;font-size:12px;line-height:1.6;color:#94a3b8;text-align:center;">
                &copy; ${new Date().getFullYear()} Conector Health &bull; Confidential clinic report
              </td>
            </tr>
          </table>
        </td>
      </tr>
    </table>
  </body>
</html>`
}

export function buildDailyVisitEmailText(context: DailyVisitEmailContext): string {
  const lines = [
    `Daily Team Huddle Visit Report`,
    `${context.clinicName} — ${context.reportDate}`,
    `Insurance: ${context.insuranceName || '—'}`,
    `Quality Program: ${context.qualityProgramName || '—'}`,
    '',
    'Daily Visit Table',
    'Patient | Insurance | Measure | Doctor | Appt Date | Value 1 | Value 2',
  ]

  if (context.rows.length === 0) {
    lines.push('No visits scheduled for today.')
  } else {
    context.rows.forEach((row) => {
      lines.push(
        [
          getTokenMid(context.huddleToken, row),
          row.ins_name || row.ins_id || '—',
          row.measure || '—',
          getDoctorName(row) || '—',
          getApptDate(row),
          getRowValue1(row),
          getRowValue2(row),
        ].join(' | '),
      )
    })
  }

  return lines.join('\n')
}

export function getTodayReportDateLabel(): string {
  const today = getClinicTodayDateString()
  return formatUsDate(today) || today
}
