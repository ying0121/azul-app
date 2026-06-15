import { motion } from 'framer-motion'
import { FileSpreadsheet, Filter, Mail, Palette, RefreshCw, Search } from 'lucide-react'
import clsx from 'clsx'
import { FilterSelect } from '@/components/ui/FilterSelect'
import type { InsuranceOption, QualityProgramOption } from '@/types/filters'

interface TableToolbarProps {
  title: string
  search: string
  onSearchChange: (value: string) => void
  onRefresh: () => void
  onOpenFilters: () => void
  onExportExcel: () => void
  onSendDailyEmail: () => void
  onOpenStatusColors: () => void
  hasActiveFilters?: boolean
  isRefreshing?: boolean
  isSendingEmail?: boolean
  totalCount: number
  insuranceOptions: InsuranceOption[]
  qualityProgramOptions: QualityProgramOption[]
  selectedInsuranceId: string
  selectedQualityProgramId: string
  onInsuranceChange: (insId: string) => void
  onQualityProgramChange: (qpId: string) => void
  isInsuranceLoading?: boolean
  isQualityProgramLoading?: boolean
}

export function TableToolbar({
  title,
  search,
  onSearchChange,
  onRefresh,
  onOpenFilters,
  onExportExcel,
  onSendDailyEmail,
  onOpenStatusColors,
  hasActiveFilters,
  isRefreshing,
  isSendingEmail,
  totalCount,
  insuranceOptions,
  qualityProgramOptions,
  selectedInsuranceId,
  selectedQualityProgramId,
  onInsuranceChange,
  onQualityProgramChange,
  isInsuranceLoading,
  isQualityProgramLoading,
}: TableToolbarProps) {
  return (
    <motion.div
      className="table-toolbar"
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.1 }}
    >
      <div className="table-toolbar__left">
        <h2 className="table-toolbar__title">{title}</h2>
        <span className="table-toolbar__count">{totalCount} records</span>
      </div>

      <div className="table-toolbar__right">
        <div className="table-toolbar__filters">
          <FilterSelect
            value={selectedInsuranceId}
            onChange={onInsuranceChange}
            options={insuranceOptions.map((option) => ({
              value: option.ins_id,
              label: option.ins_name,
            }))}
            placeholder="Insurance"
            disabled={isInsuranceLoading}
            loading={isInsuranceLoading}
            ariaLabel="Filter by insurance"
          />

          <FilterSelect
            value={selectedQualityProgramId}
            onChange={onQualityProgramChange}
            options={qualityProgramOptions.map((option) => ({
              value: option.qp_id,
              label: option.qp_name,
            }))}
            placeholder="Quality program"
            disabled={!selectedInsuranceId || isQualityProgramLoading}
            loading={isQualityProgramLoading}
            ariaLabel="Filter by quality program"
          />
        </div>

        <div className="table-toolbar__search">
          <Search size={18} className="table-toolbar__search-icon" />
          <input
            type="search"
            placeholder="Search patients, clinics, measures..."
            value={search}
            onChange={(e) => onSearchChange(e.target.value)}
            className="table-toolbar__search-input"
            aria-label="Search table"
          />
        </div>

        <div className="table-toolbar__actions">
          <button
            className="toolbar-icon-btn"
            title="Export to Excel"
            onClick={onExportExcel}
            aria-label="Export to Excel"
          >
            <FileSpreadsheet size={18} />
          </button>
          <button
            className="toolbar-icon-btn"
            title="Preview and send daily email"
            onClick={onSendDailyEmail}
            disabled={isSendingEmail || isRefreshing}
            aria-label="Preview and send daily email"
          >
            <Mail size={18} className={isSendingEmail ? 'spin' : ''} />
          </button>
          <button
            className="toolbar-icon-btn"
            title="View status colors"
            onClick={onOpenStatusColors}
            aria-label="View status colors"
          >
            <Palette size={18} />
          </button>
          <button
            className={clsx('toolbar-icon-btn', hasActiveFilters && 'toolbar-icon-btn--active')}
            title="Search filters"
            onClick={onOpenFilters}
            aria-label="Open search filters"
          >
            <Filter size={18} />
            {hasActiveFilters && <span className="toolbar-icon-btn__dot" />}
          </button>
          <button
            className="toolbar-icon-btn"
            title="Refresh data"
            onClick={onRefresh}
            disabled={isRefreshing}
          >
            <RefreshCw size={18} className={isRefreshing ? 'spin' : ''} />
          </button>
        </div>
      </div>
    </motion.div>
  )
}
