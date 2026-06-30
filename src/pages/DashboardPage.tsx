import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  fetchInsuranceList,
  fetchMeasureList,
  fetchPcpList,
  fetchQualityPrograms,
} from '@/api/filters'
import { prepareDailyVisitEmail, sendDailyEmail, type DailyEmailPreviewData } from '@/api/email'
import { exportPatientsToExcel } from '@/lib/exportTableExcel'
import { getClinicTodayDateString } from '@/lib/clinicDate'
import { loadSavedFilters, saveFilters } from '@/lib/filterStorage'
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
import { getClinicAcronym, getClinicDisplayName, getClinicId } from '@/types/auth'
import {
  ALL_INSURANCES_ID,
  ALL_INSURANCES_OPTION,
  ALL_PCPS_ID,
  ALL_PCPS_OPTION,
  ALL_QUALITY_PROGRAM_ID,
  ALL_QUALITY_PROGRAM_OPTION,
  DEFAULT_APPT_FILTER,
  DEFAULT_SOURCE_FILTER,
  formatApptFilterLabel,
  formatFilterStatusLabel,
  formatMeasureFilterLabel,
  formatMeasuresParam,
  formatSourceFilterLabel,
  getCurrentYear,
  hasNonDefaultFilters,
  matchesSourceFilter,
  resolveApptRange,
  type ApptFilterState,
  type InsuranceOption,
  type MeasureOption,
  type PcpOption,
  type QualityProgramOption,
  type SourceFilterState,
} from '@/types/filters'

function stableClinicId(clinic: ReturnType<typeof useAuthStore.getState>['clinic']) {
  const id = getClinicId(clinic)
  return id == null ? null : String(id)
}

function arraysEqual(a: string[], b: string[]) {
  if (a.length !== b.length) return false
  const sortedA = [...a].sort()
  const sortedB = [...b].sort()
  return sortedA.every((value, index) => value === sortedB[index])
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
  const [qualityProgramInsuranceId, setQualityProgramInsuranceId] = useState<string | null>(null)
  const [modalQualityProgramOptions, setModalQualityProgramOptions] = useState<QualityProgramOption[]>(
    [],
  )
  const [modalQualityProgramInsuranceId, setModalQualityProgramInsuranceId] = useState<
    string | null
  >(null)
  const [pcpOptions, setPcpOptions] = useState<PcpOption[]>([])
  const [measureOptions, setMeasureOptions] = useState<MeasureOption[]>([])

  const [appliedInsuranceId, setAppliedInsuranceId] = useState(ALL_INSURANCES_ID)
  const [appliedQualityProgramId, setAppliedQualityProgramId] = useState(ALL_QUALITY_PROGRAM_ID)
  const [appliedPcpId, setAppliedPcpId] = useState(ALL_PCPS_ID)
  const [appliedMeasureIds, setAppliedMeasureIds] = useState<string[]>([])
  const [appliedApptFilter, setAppliedApptFilter] = useState<ApptFilterState>(DEFAULT_APPT_FILTER)
  const [appliedSourceFilter, setAppliedSourceFilter] =
    useState<SourceFilterState>(DEFAULT_SOURCE_FILTER)

  const [draftInsuranceId, setDraftInsuranceId] = useState(ALL_INSURANCES_ID)
  const [draftQualityProgramId, setDraftQualityProgramId] = useState(ALL_QUALITY_PROGRAM_ID)
  const [draftPcpId, setDraftPcpId] = useState(ALL_PCPS_ID)
  const [draftMeasureIds, setDraftMeasureIds] = useState<string[]>([])
  const [draftApptFilter, setDraftApptFilter] = useState<ApptFilterState>(DEFAULT_APPT_FILTER)
  const [draftSourceFilter, setDraftSourceFilter] =
    useState<SourceFilterState>(DEFAULT_SOURCE_FILTER)

  const [search, setSearch] = useState('')
  const [isFilterModalOpen, setIsFilterModalOpen] = useState(false)
  const [isStatusColorModalOpen, setIsStatusColorModalOpen] = useState(false)
  const [isLoading, setIsLoading] = useState(false)
  const [isRefreshing, setIsRefreshing] = useState(false)
  const [isInsuranceLoading, setIsInsuranceLoading] = useState(false)
  const [isQualityProgramLoading, setIsQualityProgramLoading] = useState(false)
  const [isModalQualityProgramLoading, setIsModalQualityProgramLoading] = useState(false)
  const [isPcpLoading, setIsPcpLoading] = useState(false)
  const [isMeasureLoading, setIsMeasureLoading] = useState(false)
  const [isSendingEmail, setIsSendingEmail] = useState(false)
  const [isEmailPreviewOpen, setIsEmailPreviewOpen] = useState(false)
  const [isEmailPreviewLoading, setIsEmailPreviewLoading] = useState(false)
  const [emailPreview, setEmailPreview] = useState<DailyEmailPreviewData | null>(null)

  const showAlertRef = useRef(showAlert)
  const appliedInsuranceIdRef = useRef(appliedInsuranceId)
  const appliedQualityProgramIdRef = useRef(appliedQualityProgramId)
  const appliedPcpIdRef = useRef(appliedPcpId)
  const appliedMeasureIdsRef = useRef(appliedMeasureIds)
  const searchRef = useRef(search)
  const appliedApptFilterRef = useRef(appliedApptFilter)
  const loadedInsuranceForClinic = useRef<string | null>(null)
  const loadedQualityForInsurance = useRef<string | null>(null)
  const loadedModalQualityForInsurance = useRef<string | null>(null)
  const loadedPcpForClinic = useRef<string | null>(null)
  const loadedMeasureForClinic = useRef<string | null>(null)
  const hasLoadedPatientsOnce = useRef(false)

  showAlertRef.current = showAlert
  appliedInsuranceIdRef.current = appliedInsuranceId
  appliedQualityProgramIdRef.current = appliedQualityProgramId
  appliedPcpIdRef.current = appliedPcpId
  appliedMeasureIdsRef.current = appliedMeasureIds
  searchRef.current = search
  appliedApptFilterRef.current = appliedApptFilter

  const applySavedFilters = useCallback((saved: ReturnType<typeof loadSavedFilters>) => {
    setAppliedInsuranceId(saved.insuranceId)
    setAppliedQualityProgramId(saved.qualityProgramId)
    setAppliedPcpId(saved.pcpId)
    setAppliedMeasureIds(saved.measureIds)
    setAppliedApptFilter(saved.apptFilter)
    setAppliedSourceFilter(saved.sourceFilter)
    appliedInsuranceIdRef.current = saved.insuranceId
    appliedQualityProgramIdRef.current = saved.qualityProgramId
    appliedPcpIdRef.current = saved.pcpId
    appliedMeasureIdsRef.current = saved.measureIds
    appliedApptFilterRef.current = saved.apptFilter
  }, [])

  const buildPatientFilters = useCallback((activeClinicId: string) => {
    const { appt_start, appt_end } = resolveApptRange(appliedApptFilterRef.current)

    return {
      clinic_id: activeClinicId,
      ins_id: appliedInsuranceIdRef.current,
      qp_id: appliedQualityProgramIdRef.current,
      pcp_id: appliedPcpIdRef.current || ALL_PCPS_ID,
      cyear: getCurrentYear(),
      filter: searchRef.current.trim(),
      measures: formatMeasuresParam(appliedMeasureIdsRef.current),
      appt_start,
      appt_end,
    }
  }, [])

  const loadPatients = useCallback(async (refresh = false) => {
    const activeClinicId = stableClinicId(useAuthStore.getState().clinic)
    if (activeClinicId == null) return
    if (!appliedInsuranceIdRef.current || !appliedQualityProgramIdRef.current) return

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
      setInsuranceOptions([ALL_INSURANCES_OPTION, ...options])
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
      if (loadedQualityForInsurance.current === cacheKey) {
        setQualityProgramInsuranceId(insId)
        return
      }

      setQualityProgramInsuranceId(null)
      setIsQualityProgramLoading(true)
      setQualityProgramOptions([])

      try {
        const options = await fetchQualityPrograms(activeClinicId, insId)
        setQualityProgramOptions([ALL_QUALITY_PROGRAM_OPTION, ...options])
        loadedQualityForInsurance.current = cacheKey
        setQualityProgramInsuranceId(insId)
      } catch (err: unknown) {
        showAlertRef.current(
          'error',
          'Load Failed',
          (err as { friendlyMessage?: string }).friendlyMessage ??
            'Unable to load quality programs.',
        )
        setQualityProgramOptions([ALL_QUALITY_PROGRAM_OPTION])
        loadedQualityForInsurance.current = cacheKey
        setQualityProgramInsuranceId(insId)
      } finally {
        setIsQualityProgramLoading(false)
      }
    },
    [],
  )

  const loadModalQualityProgramOptions = useCallback(
    async (activeClinicId: string, insId: string) => {
      const cacheKey = `${activeClinicId}:${insId}`
      if (loadedModalQualityForInsurance.current === cacheKey) {
        setModalQualityProgramInsuranceId(insId)
        return
      }

      setModalQualityProgramInsuranceId(null)
      setIsModalQualityProgramLoading(true)
      setModalQualityProgramOptions([])

      try {
        const options = await fetchQualityPrograms(activeClinicId, insId)
        setModalQualityProgramOptions([ALL_QUALITY_PROGRAM_OPTION, ...options])
        loadedModalQualityForInsurance.current = cacheKey
        setModalQualityProgramInsuranceId(insId)
      } catch (err: unknown) {
        showAlertRef.current(
          'error',
          'Load Failed',
          (err as { friendlyMessage?: string }).friendlyMessage ??
            'Unable to load quality programs.',
        )
        setModalQualityProgramOptions([ALL_QUALITY_PROGRAM_OPTION])
        loadedModalQualityForInsurance.current = cacheKey
        setModalQualityProgramInsuranceId(insId)
      } finally {
        setIsModalQualityProgramLoading(false)
      }
    },
    [],
  )

  const loadPcpOptions = useCallback(async (activeClinicId: string) => {
    if (loadedPcpForClinic.current === activeClinicId) return

    setIsPcpLoading(true)
    try {
      const options = await fetchPcpList(activeClinicId)
      setPcpOptions([ALL_PCPS_OPTION, ...options])
    } catch (err: unknown) {
      showAlertRef.current(
        'error',
        'Load Failed',
        (err as { friendlyMessage?: string }).friendlyMessage ??
          'Unable to load PCP options.',
      )
    } finally {
      loadedPcpForClinic.current = activeClinicId
      setIsPcpLoading(false)
    }
  }, [])

  const loadMeasureOptions = useCallback(async (activeClinicId: string) => {
    if (loadedMeasureForClinic.current === activeClinicId) return

    setIsMeasureLoading(true)
    try {
      const options = await fetchMeasureList(activeClinicId)
      setMeasureOptions(options)
    } catch (err: unknown) {
      showAlertRef.current(
        'error',
        'Load Failed',
        (err as { friendlyMessage?: string }).friendlyMessage ??
          'Unable to load measure options.',
      )
    } finally {
      loadedMeasureForClinic.current = activeClinicId
      setIsMeasureLoading(false)
    }
  }, [])

  useEffect(() => {
    if (clinicId == null) return
    void loadStatusColors()
  }, [clinicId, loadStatusColors])

  useEffect(() => {
    if (clinicId == null) return

    loadedInsuranceForClinic.current = null
    loadedQualityForInsurance.current = null
    loadedModalQualityForInsurance.current = null
    loadedPcpForClinic.current = null
    loadedMeasureForClinic.current = null
    hasLoadedPatientsOnce.current = false

    setInsuranceOptions([])
    setQualityProgramOptions([])
    setQualityProgramInsuranceId(null)
    setModalQualityProgramOptions([])
    setModalQualityProgramInsuranceId(null)
    setPcpOptions([])
    setMeasureOptions([])
    setPatients([])

    const saved = loadSavedFilters(clinicId)
    applySavedFilters(saved)

    void loadInsuranceOptions(clinicId)
    void loadPcpOptions(clinicId)
    void loadMeasureOptions(clinicId)
  }, [
    clinicId,
    applySavedFilters,
    loadInsuranceOptions,
    loadPcpOptions,
    loadMeasureOptions,
  ])

  useEffect(() => {
    if (insuranceOptions.length === 0 || isInsuranceLoading) return

    const isValid = insuranceOptions.some((option) => option.ins_id === appliedInsuranceId)
    if (!isValid) {
      setAppliedInsuranceId(insuranceOptions[0].ins_id)
    }
  }, [insuranceOptions, appliedInsuranceId, isInsuranceLoading])

  useEffect(() => {
    if (clinicId == null || !appliedInsuranceId || isInsuranceLoading) return

    void loadQualityProgramOptions(clinicId, appliedInsuranceId)
  }, [clinicId, appliedInsuranceId, isInsuranceLoading, loadQualityProgramOptions])

  useEffect(() => {
    if (qualityProgramOptions.length === 0 || isQualityProgramLoading) return
    if (qualityProgramInsuranceId !== appliedInsuranceId) return

    const isValid = qualityProgramOptions.some(
      (option) => option.qp_id === appliedQualityProgramId,
    )
    if (!isValid) {
      setAppliedQualityProgramId(qualityProgramOptions[0].qp_id)
    }
  }, [
    qualityProgramOptions,
    appliedQualityProgramId,
    appliedInsuranceId,
    qualityProgramInsuranceId,
    isQualityProgramLoading,
  ])

  useEffect(() => {
    if (pcpOptions.length === 0 || isPcpLoading) return

    const isValid = pcpOptions.some((option) => option.pcp_id === appliedPcpId)
    if (!isValid) {
      setAppliedPcpId(pcpOptions[0].pcp_id)
    }
  }, [pcpOptions, appliedPcpId, isPcpLoading])

  useEffect(() => {
    if (measureOptions.length === 0 || appliedMeasureIds.length === 0) return

    const validIds = new Set(measureOptions.map((option) => option.measure_id))
    const nextIds = appliedMeasureIds.filter((id) => validIds.has(id))
    if (!arraysEqual(nextIds, appliedMeasureIds)) {
      setAppliedMeasureIds(nextIds)
    }
  }, [measureOptions, appliedMeasureIds])

  const areFilterOptionsReady = useMemo(() => {
    if (clinicId == null) return false
    if (isInsuranceLoading || isQualityProgramLoading || isPcpLoading || isMeasureLoading) {
      return false
    }
    if (insuranceOptions.length === 0 || qualityProgramOptions.length === 0 || pcpOptions.length === 0) {
      return false
    }
    if (qualityProgramInsuranceId !== appliedInsuranceId) return false

    return (
      insuranceOptions.some((option) => option.ins_id === appliedInsuranceId) &&
      qualityProgramOptions.some((option) => option.qp_id === appliedQualityProgramId) &&
      pcpOptions.some((option) => option.pcp_id === appliedPcpId)
    )
  }, [
    clinicId,
    isInsuranceLoading,
    isQualityProgramLoading,
    isPcpLoading,
    isMeasureLoading,
    insuranceOptions,
    qualityProgramOptions,
    pcpOptions,
    appliedInsuranceId,
    appliedQualityProgramId,
    appliedPcpId,
    qualityProgramInsuranceId,
  ])

  useEffect(() => {
    if (!areFilterOptionsReady) return

    void loadPatients(hasLoadedPatientsOnce.current)
  }, [
    areFilterOptionsReady,
    appliedInsuranceId,
    appliedQualityProgramId,
    appliedPcpId,
    appliedApptFilter,
    appliedMeasureIds,
    loadPatients,
  ])

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
    void exportPatientsToExcel(rows, filename).then(() => {
      showAlert('success', 'Export Complete', `Exported ${rows.length} record(s) to ${filename}.`)
    })
  }

  const handleOpenDailyEmailPreview = async () => {
    if (clinicId == null || !appliedInsuranceId || !appliedQualityProgramId) {
      showAlert('warning', 'Email Not Available', 'Select insurance and quality program first.')
      return
    }

    setIsEmailPreviewOpen(true)
    setIsEmailPreviewLoading(true)
    setEmailPreview(null)

    try {
      const preview = await prepareDailyVisitEmail({
        clinicId,
        clinicAcronym: getClinicAcronym(clinic),
        insId: appliedInsuranceId,
        qpId: appliedQualityProgramId,
        clinicName: getClinicDisplayName(clinic),
        insuranceName:
          insuranceOptions.find((option) => option.ins_id === appliedInsuranceId)?.ins_name ??
          appliedInsuranceId,
        qualityProgramName:
          qualityProgramOptions.find((option) => option.qp_id === appliedQualityProgramId)
            ?.qp_name ?? appliedQualityProgramId,
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
    setDraftInsuranceId(appliedInsuranceId)
    setDraftQualityProgramId(appliedQualityProgramId)
    setDraftPcpId(appliedPcpId)
    setDraftMeasureIds(appliedMeasureIds)
    setDraftApptFilter(appliedApptFilter)
    setDraftSourceFilter(appliedSourceFilter)

    if (clinicId != null && qualityProgramInsuranceId === appliedInsuranceId) {
      setModalQualityProgramOptions(qualityProgramOptions)
      setModalQualityProgramInsuranceId(appliedInsuranceId)
      loadedModalQualityForInsurance.current = `${clinicId}:${appliedInsuranceId}`
    } else {
      setModalQualityProgramOptions([])
      setModalQualityProgramInsuranceId(null)
      loadedModalQualityForInsurance.current = null
    }

    setIsFilterModalOpen(true)
  }

  const handleDraftInsuranceChange = (insId: string) => {
    if (!insId || insId === draftInsuranceId) return
    setDraftInsuranceId(insId)
    setDraftQualityProgramId(ALL_QUALITY_PROGRAM_ID)
    setModalQualityProgramOptions([])
    setModalQualityProgramInsuranceId(null)
    loadedModalQualityForInsurance.current = null
    if (clinicId != null) {
      void loadModalQualityProgramOptions(clinicId, insId)
    }
  }

  useEffect(() => {
    if (!isFilterModalOpen || isModalQualityProgramLoading || clinicId == null) return
    if (modalQualityProgramInsuranceId === draftInsuranceId) return
    void loadModalQualityProgramOptions(clinicId, draftInsuranceId)
  }, [
    isFilterModalOpen,
    isModalQualityProgramLoading,
    clinicId,
    draftInsuranceId,
    modalQualityProgramInsuranceId,
    loadModalQualityProgramOptions,
  ])

  useEffect(() => {
    if (!isFilterModalOpen || isModalQualityProgramLoading) return
    if (modalQualityProgramOptions.length === 0) return
    if (modalQualityProgramInsuranceId !== draftInsuranceId) return

    const isValid = modalQualityProgramOptions.some(
      (option) => option.qp_id === draftQualityProgramId,
    )
    if (!isValid) {
      setDraftQualityProgramId(modalQualityProgramOptions[0].qp_id)
    }
  }, [
    isFilterModalOpen,
    isModalQualityProgramLoading,
    modalQualityProgramOptions,
    draftQualityProgramId,
    draftInsuranceId,
    modalQualityProgramInsuranceId,
  ])

  const handleApplyFilters = () => {
    const nextInsuranceId = draftInsuranceId
    const nextQualityProgramId = draftQualityProgramId

    setAppliedInsuranceId(nextInsuranceId)
    setAppliedQualityProgramId(nextQualityProgramId)
    setAppliedPcpId(draftPcpId)
    setAppliedMeasureIds(draftMeasureIds)
    setAppliedApptFilter(draftApptFilter)
    setAppliedSourceFilter(draftSourceFilter)
    appliedInsuranceIdRef.current = nextInsuranceId
    appliedQualityProgramIdRef.current = nextQualityProgramId
    appliedPcpIdRef.current = draftPcpId
    appliedMeasureIdsRef.current = draftMeasureIds
    appliedApptFilterRef.current = draftApptFilter
    setIsFilterModalOpen(false)

    if (
      clinicId != null &&
      modalQualityProgramInsuranceId === nextInsuranceId &&
      modalQualityProgramOptions.length > 0
    ) {
      setQualityProgramOptions(modalQualityProgramOptions)
      setQualityProgramInsuranceId(nextInsuranceId)
      loadedQualityForInsurance.current = `${clinicId}:${nextInsuranceId}`
    } else if (clinicId != null) {
      loadedQualityForInsurance.current = null
      setQualityProgramInsuranceId(null)
    }

    if (clinicId != null) {
      saveFilters(clinicId, {
        insuranceId: nextInsuranceId,
        qualityProgramId: nextQualityProgramId,
        pcpId: draftPcpId,
        measureIds: draftMeasureIds,
        sourceFilter: draftSourceFilter,
        apptFilter: draftApptFilter,
      })
    }
  }

  const filteredPatientCount = useMemo(
    () => patients.filter((row) => matchesSourceFilter(row.source, appliedSourceFilter)).length,
    [patients, appliedSourceFilter],
  )

  const filterStatusLabel = useMemo(
    () =>
      formatFilterStatusLabel({
        insuranceName:
          insuranceOptions.find((option) => option.ins_id === appliedInsuranceId)?.ins_name ??
          ALL_INSURANCES_OPTION.ins_name,
        qualityProgramName:
          qualityProgramOptions.find((option) => option.qp_id === appliedQualityProgramId)
            ?.qp_name ?? ALL_QUALITY_PROGRAM_OPTION.qp_name,
        pcpName:
          pcpOptions.find((option) => option.pcp_id === appliedPcpId)?.pcp_name ??
          ALL_PCPS_OPTION.pcp_name,
        measureLabel: formatMeasureFilterLabel(appliedMeasureIds, measureOptions),
        sourceLabel: formatSourceFilterLabel(appliedSourceFilter),
        apptLabel: formatApptFilterLabel(appliedApptFilter),
      }),
    [
      insuranceOptions,
      appliedInsuranceId,
      qualityProgramOptions,
      appliedQualityProgramId,
      pcpOptions,
      appliedPcpId,
      measureOptions,
      appliedMeasureIds,
      appliedSourceFilter,
      appliedApptFilter,
    ],
  )

  const hasActiveFilters = hasNonDefaultFilters({
    insuranceId: appliedInsuranceId,
    qualityProgramId: appliedQualityProgramId,
    pcpId: appliedPcpId,
    measureIds: appliedMeasureIds,
    sourceFilter: appliedSourceFilter,
    apptFilter: appliedApptFilter,
  })

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
          filterStatusLabel={filterStatusLabel}
          hasActiveFilters={hasActiveFilters}
          isRefreshing={isRefreshing}
          isSendingEmail={isEmailPreviewLoading || isSendingEmail}
          totalCount={filteredPatientCount}
        />

        <DataTable
          data={patients}
          globalFilter={search}
          sourceFilter={appliedSourceFilter}
          isLoading={isLoading || !areFilterOptionsReady}
        />
      </main>

      <StatusColorModal
        open={isStatusColorModalOpen}
        onClose={() => setIsStatusColorModalOpen(false)}
      />

      <FilterModal
        open={isFilterModalOpen}
        onClose={() => setIsFilterModalOpen(false)}
        draftInsuranceId={draftInsuranceId}
        draftQualityProgramId={draftQualityProgramId}
        draftPcpId={draftPcpId}
        draftMeasureIds={draftMeasureIds}
        draftAppt={draftApptFilter}
        draftSource={draftSourceFilter}
        insuranceOptions={insuranceOptions}
        qualityProgramOptions={modalQualityProgramOptions}
        pcpOptions={pcpOptions}
        measureOptions={measureOptions}
        onDraftInsuranceChange={handleDraftInsuranceChange}
        onDraftQualityProgramChange={setDraftQualityProgramId}
        onDraftPcpChange={setDraftPcpId}
        onDraftMeasureIdsChange={setDraftMeasureIds}
        onDraftApptChange={setDraftApptFilter}
        onDraftSourceChange={setDraftSourceFilter}
        isInsuranceLoading={isInsuranceLoading}
        isQualityProgramLoading={isModalQualityProgramLoading}
        isPcpLoading={isPcpLoading}
        isMeasureLoading={isMeasureLoading}
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
