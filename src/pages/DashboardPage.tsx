import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { fetchInsuranceList, fetchQualityPrograms } from '@/api/filters'
import { prepareDailyVisitEmail, sendDailyEmail, type DailyEmailPreviewData } from '@/api/email'
import { exportPatientsToExcel } from '@/lib/exportTableExcel'
import { getClinicTodayDateString } from '@/lib/clinicDate'
import { filterDisplayedPatients } from '@/lib/patientTableFilter'
import { fetchPatients } from '@/api/patients'
import type { PatientRow } from '@/types/patient'
import { DailyEmailPreviewModal } from '@/components/dashboard/DailyEmailPreviewModal'
import { FilterModal } from '@/components/dashboard/FilterModal'
import { StatusColorModal } from '@/components/dashboard/StatusColorModal'
import { Header } from '@/components/layout/Header'
import { DataTable } from '@/components/dashboard/DataTable'
import { TableToolbar } from '@/components/dashboard/TableToolbar'
import { useAlertStore } from '@/stores/alertStore'
import { useAuthStore } from '@/stores/authStore'
import { useStatusColorStore } from '@/stores/statusColorStore'
import { getClinicDisplayName, getClinicId } from '@/types/auth'
import {
  DEFAULT_APPT_FILTER,
  DEFAULT_SOURCE_FILTER,
  getCurrentYear,
  isDefaultSourceFilter,
  matchesSourceFilter,
  resolveApptRange,
  type ApptFilterState,
  type InsuranceOption,
  type QualityProgramOption,
  type SourceFilterState,
} from '@/types/filters'

function stableClinicId(clinic: ReturnType<typeof useAuthStore.getState>['clinic']) {
  const id = getClinicId(clinic)
  return id == null ? null : String(id)
}

export function DashboardPage() {
  const showAlert = useAlertStore((s) => s.show)
  const clinic = useAuthStore((s) => s.clinic)
  const loadStatusColors = useStatusColorStore((s) => s.loadStatusColors)
  const hedisStatus = useStatusColorStore((s) => s.hedisStatus)
  const clinicId = stableClinicId(clinic)

  const [patients, setPatients] = useState<PatientRow[]>([])
  const [insuranceOptions, setInsuranceOptions] = useState<InsuranceOption[]>([])
  const [qualityProgramOptions, setQualityProgramOptions] = useState<QualityProgramOption[]>([])
  const [selectedInsuranceId, setSelectedInsuranceId] = useState('')
  const [selectedQualityProgramId, setSelectedQualityProgramId] = useState('')
  const [search, setSearch] = useState('')
  const [appliedApptFilter, setAppliedApptFilter] = useState<ApptFilterState>(DEFAULT_APPT_FILTER)
  const [draftApptFilter, setDraftApptFilter] = useState<ApptFilterState>(DEFAULT_APPT_FILTER)
  const [appliedSourceFilter, setAppliedSourceFilter] =
    useState<SourceFilterState>(DEFAULT_SOURCE_FILTER)
  const [draftSourceFilter, setDraftSourceFilter] =
    useState<SourceFilterState>(DEFAULT_SOURCE_FILTER)
  const [isFilterModalOpen, setIsFilterModalOpen] = useState(false)
  const [isStatusColorModalOpen, setIsStatusColorModalOpen] = useState(false)
  const [isLoading, setIsLoading] = useState(false)
  const [isRefreshing, setIsRefreshing] = useState(false)
  const [isInsuranceLoading, setIsInsuranceLoading] = useState(false)
  const [isQualityProgramLoading, setIsQualityProgramLoading] = useState(false)
  const [isSendingEmail, setIsSendingEmail] = useState(false)
  const [isEmailPreviewOpen, setIsEmailPreviewOpen] = useState(false)
  const [isEmailPreviewLoading, setIsEmailPreviewLoading] = useState(false)
  const [emailPreview, setEmailPreview] = useState<DailyEmailPreviewData | null>(null)

  const showAlertRef = useRef(showAlert)
  const selectedInsuranceIdRef = useRef(selectedInsuranceId)
  const selectedQualityProgramIdRef = useRef(selectedQualityProgramId)
  const searchRef = useRef(search)
  const appliedApptFilterRef = useRef(appliedApptFilter)
  const loadedInsuranceForClinic = useRef<string | null>(null)
  const loadedQualityForInsurance = useRef<string | null>(null)
  const hasLoadedPatientsOnce = useRef(false)

  showAlertRef.current = showAlert
  selectedInsuranceIdRef.current = selectedInsuranceId
  selectedQualityProgramIdRef.current = selectedQualityProgramId
  searchRef.current = search
  appliedApptFilterRef.current = appliedApptFilter

  const buildPatientFilters = useCallback((activeClinicId: string) => {
    const { appt_start, appt_end } = resolveApptRange(appliedApptFilterRef.current)

    return {
      clinic_id: activeClinicId,
      ins_id: selectedInsuranceIdRef.current,
      qp_id: selectedQualityProgramIdRef.current,
      cyear: getCurrentYear(),
      filter: searchRef.current.trim(),
      appt_start,
      appt_end,
    }
  }, [])

  const loadPatients = useCallback(async (refresh = false) => {
    const activeClinicId = stableClinicId(useAuthStore.getState().clinic)
    if (activeClinicId == null) return
    if (!selectedInsuranceIdRef.current || !selectedQualityProgramIdRef.current) return

    const showRefresh = refresh || hasLoadedPatientsOnce.current
    if (showRefresh) setIsRefreshing(true)
    else setIsLoading(true)

    try {
      const { rows } = await fetchPatients(buildPatientFilters(activeClinicId))
      setPatients(rows)
      hasLoadedPatientsOnce.current = true
    } catch (err: unknown) {
      showAlertRef.current(
        'error',
        'Load Failed',
        (err as { friendlyMessage?: string }).friendlyMessage ??
          'Unable to load patient data.',
      )
    } finally {
      setIsLoading(false)
      setIsRefreshing(false)
    }
  }, [buildPatientFilters])

  const loadInsuranceOptions = useCallback(async (activeClinicId: string) => {
    if (loadedInsuranceForClinic.current === activeClinicId) return

    setIsInsuranceLoading(true)
    try {
      const options = await fetchInsuranceList(activeClinicId)
      setInsuranceOptions(options)
    } catch (err: unknown) {
      showAlertRef.current(
        'error',
        'Load Failed',
        (err as { friendlyMessage?: string }).friendlyMessage ??
          'Unable to load insurance options.',
      )
    } finally {
      loadedInsuranceForClinic.current = activeClinicId
      setIsInsuranceLoading(false)
    }
  }, [])

  const loadQualityProgramOptions = useCallback(
    async (activeClinicId: string, insId: string) => {
      const cacheKey = `${activeClinicId}:${insId}`
      if (loadedQualityForInsurance.current === cacheKey) return

      setIsQualityProgramLoading(true)
      try {
        const options = await fetchQualityPrograms(activeClinicId, insId)
        setQualityProgramOptions(options)
        loadedQualityForInsurance.current = cacheKey
      } catch (err: unknown) {
        showAlertRef.current(
          'error',
          'Load Failed',
          (err as { friendlyMessage?: string }).friendlyMessage ??
            'Unable to load quality programs.',
        )
        setQualityProgramOptions([])
      } finally {
        setIsQualityProgramLoading(false)
      }
    },
    [],
  )

  useEffect(() => {
    if (clinicId == null) return
    void loadStatusColors()
  }, [clinicId, loadStatusColors])

  useEffect(() => {
    if (clinicId == null) return

    loadedInsuranceForClinic.current = null
    loadedQualityForInsurance.current = null
    hasLoadedPatientsOnce.current = false

    setSelectedInsuranceId('')
    setSelectedQualityProgramId('')
    setInsuranceOptions([])
    setQualityProgramOptions([])
    setPatients([])

    void loadInsuranceOptions(clinicId)
  }, [clinicId, loadInsuranceOptions])

  useEffect(() => {
    if (insuranceOptions.length === 0) return

    const isValid = insuranceOptions.some((option) => option.ins_id === selectedInsuranceId)
    if (!isValid) {
      setSelectedInsuranceId(insuranceOptions[0].ins_id)
    }
  }, [insuranceOptions, selectedInsuranceId])

  useEffect(() => {
    if (clinicId == null) return

    if (!selectedInsuranceId) {
      setQualityProgramOptions([])
      setSelectedQualityProgramId('')
      loadedQualityForInsurance.current = null
      return
    }

    setQualityProgramOptions([])
    setSelectedQualityProgramId('')
    loadedQualityForInsurance.current = null
    void loadQualityProgramOptions(clinicId, selectedInsuranceId)
  }, [clinicId, selectedInsuranceId, loadQualityProgramOptions])

  useEffect(() => {
    if (qualityProgramOptions.length === 0) return

    const isValid = qualityProgramOptions.some(
      (option) => option.qp_id === selectedQualityProgramId,
    )
    if (!isValid) {
      setSelectedQualityProgramId(qualityProgramOptions[0].qp_id)
    }
  }, [qualityProgramOptions, selectedQualityProgramId])

  useEffect(() => {
    if (clinicId == null || !selectedInsuranceId || !selectedQualityProgramId) return

    void loadPatients(hasLoadedPatientsOnce.current)
  }, [clinicId, selectedInsuranceId, selectedQualityProgramId, loadPatients])

  const handleInsuranceChange = (insId: string) => {
    if (!insId || insId === selectedInsuranceId) return
    setSelectedInsuranceId(insId)
  }

  const handleQualityProgramChange = (qpId: string) => {
    if (!qpId || qpId === selectedQualityProgramId) return
    setSelectedQualityProgramId(qpId)
  }

  const handleRefresh = () => {
    void loadPatients(true)
  }

  const handleExportExcel = () => {
    const rows = filterDisplayedPatients(patients, appliedSourceFilter, search)

    if (rows.length === 0) {
      showAlert('warning', 'Export Excel', 'No records to export.')
      return
    }

    const filename = `daily-huddle-${getClinicTodayDateString()}.xlsx`
    exportPatientsToExcel(rows, filename)
    showAlert('success', 'Export Complete', `Exported ${rows.length} record(s) to ${filename}.`)
  }

  const handleOpenDailyEmailPreview = async () => {
    if (clinicId == null || !selectedInsuranceId || !selectedQualityProgramId) {
      showAlert('warning', 'Email Not Available', 'Select insurance and quality program first.')
      return
    }

    setIsEmailPreviewOpen(true)
    setIsEmailPreviewLoading(true)
    setEmailPreview(null)

    try {
      const preview = await prepareDailyVisitEmail({
        clinicId,
        insId: selectedInsuranceId,
        qpId: selectedQualityProgramId,
        clinicName: getClinicDisplayName(clinic),
        insuranceName:
          insuranceOptions.find((option) => option.ins_id === selectedInsuranceId)?.ins_name ??
          selectedInsuranceId,
        qualityProgramName:
          qualityProgramOptions.find((option) => option.qp_id === selectedQualityProgramId)
            ?.qp_name ?? selectedQualityProgramId,
        statusMap: hedisStatus,
      })

      setEmailPreview(preview)
    } catch (err: unknown) {
      setIsEmailPreviewOpen(false)
      showAlert(
        'error',
        'Preview Failed',
        (err as { friendlyMessage?: string }).friendlyMessage ??
          'Unable to build the daily visit email preview.',
      )
    } finally {
      setIsEmailPreviewLoading(false)
    }
  }

  const handleCloseDailyEmailPreview = () => {
    if (isSendingEmail) return
    setIsEmailPreviewOpen(false)
    setEmailPreview(null)
  }

  const handleConfirmDailyEmail = async () => {
    if (!emailPreview) return

    setIsSendingEmail(true)
    try {
      const message = await sendDailyEmail(emailPreview.payload)
      setIsEmailPreviewOpen(false)
      setEmailPreview(null)
      showAlert('success', 'Daily Email Sent', message)
    } catch (err: unknown) {
      showAlert(
        'error',
        'Email Failed',
        (err as { friendlyMessage?: string }).friendlyMessage ??
          'Unable to send the daily visit email.',
      )
    } finally {
      setIsSendingEmail(false)
    }
  }

  const handleOpenStatusColors = () => {
    void loadStatusColors()
    setIsStatusColorModalOpen(true)
  }

  const handleOpenFilterModal = () => {
    setDraftApptFilter(appliedApptFilter)
    setDraftSourceFilter(appliedSourceFilter)
    setIsFilterModalOpen(true)
  }

  const handleApplyFilters = () => {
    const apptChanged =
      draftApptFilter.preset !== appliedApptFilter.preset ||
      draftApptFilter.customEndDate !== appliedApptFilter.customEndDate

    setAppliedApptFilter(draftApptFilter)
    setAppliedSourceFilter(draftSourceFilter)
    appliedApptFilterRef.current = draftApptFilter
    setIsFilterModalOpen(false)

    if (apptChanged) {
      void loadPatients(true)
    }
  }

  const filteredPatientCount = useMemo(
    () => patients.filter((row) => matchesSourceFilter(row.source, appliedSourceFilter)).length,
    [patients, appliedSourceFilter],
  )

  const hasActiveApptFilter =
    appliedApptFilter.preset !== DEFAULT_APPT_FILTER.preset ||
    (appliedApptFilter.preset === 'custom' && appliedApptFilter.customEndDate !== '')

  const hasActiveSourceFilter = !isDefaultSourceFilter(appliedSourceFilter)
  const hasActiveFilters = hasActiveApptFilter || hasActiveSourceFilter

  return (
    <div className="dashboard">
      <Header />

      <main className="dashboard__main">
        <TableToolbar
          title="Clinical Quality and Med Adherence Alerts"
          search={search}
          onSearchChange={setSearch}
          onRefresh={handleRefresh}
          onOpenFilters={handleOpenFilterModal}
          onExportExcel={handleExportExcel}
          onSendDailyEmail={() => void handleOpenDailyEmailPreview()}
          onOpenStatusColors={handleOpenStatusColors}
          hasActiveFilters={hasActiveFilters}
          isRefreshing={isRefreshing}
          isSendingEmail={isEmailPreviewLoading || isSendingEmail}
          totalCount={filteredPatientCount}
          insuranceOptions={insuranceOptions}
          qualityProgramOptions={qualityProgramOptions}
          selectedInsuranceId={selectedInsuranceId}
          selectedQualityProgramId={selectedQualityProgramId}
          onInsuranceChange={handleInsuranceChange}
          onQualityProgramChange={handleQualityProgramChange}
          isInsuranceLoading={isInsuranceLoading}
          isQualityProgramLoading={isQualityProgramLoading}
        />

        <DataTable
          data={patients}
          globalFilter={search}
          sourceFilter={appliedSourceFilter}
          isLoading={isLoading}
        />
      </main>

      <StatusColorModal
        open={isStatusColorModalOpen}
        onClose={() => setIsStatusColorModalOpen(false)}
      />

      <FilterModal
        open={isFilterModalOpen}
        onClose={() => setIsFilterModalOpen(false)}
        draft={draftApptFilter}
        draftSource={draftSourceFilter}
        onDraftChange={setDraftApptFilter}
        onDraftSourceChange={setDraftSourceFilter}
        onApply={handleApplyFilters}
      />

      <DailyEmailPreviewModal
        open={isEmailPreviewOpen}
        onClose={handleCloseDailyEmailPreview}
        payload={emailPreview?.payload ?? null}
        summary={emailPreview?.summary ?? null}
        isLoading={isEmailPreviewLoading}
        isSending={isSendingEmail}
        onConfirm={() => void handleConfirmDailyEmail()}
      />
    </div>
  )
}
