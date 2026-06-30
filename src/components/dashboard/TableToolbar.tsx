import { motion } from 'framer-motion'
import { FileSpreadsheet, Filter, Mail, Palette, RefreshCw, Search } from 'lucide-react'
import clsx from 'clsx'

interface TableToolbarProps {
  title: string
  search: string
  onSearchChange: (value: string) => void
  onRefresh: () => void
  onOpenFilters: () => void
  onExportExcel: () => void
  onSendDailyEmail: () => void
  onOpenStatusColors: () => void
  filterStatusLabel: string
  hasActiveFilters?: boolean
  isRefreshing?: boolean
  isSendingEmail?: boolean
  totalCount: number
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
  filterStatusLabel,
  hasActiveFilters,
  isRefreshing,
  isSendingEmail,
  totalCount,
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
        <button
          type="button"
          className={clsx(
            'table-toolbar__filter-status',
            hasActiveFilters && 'table-toolbar__filter-status--active',
          )}
          onClick={onOpenFilters}
          title={filterStatusLabel}
          aria-label={`Filters: ${filterStatusLabel}. Click to edit.`}
        >
          <Filter size={16} className="table-toolbar__filter-status-icon" />
          <span className="table-toolbar__filter-status-text">{filterStatusLabel}</span>
        </button>

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
            title="Edit filters"
            onClick={onOpenFilters}
            aria-label="Open filters"
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
