import { useEffect, useMemo, useState, type CSSProperties } from 'react'
import {
  flexRender,
  getCoreRowModel,
  getPaginationRowModel,
  getSortedRowModel,
  useReactTable,
  type ColumnDef,
  type PaginationState,
  type SortingState,
} from '@tanstack/react-table'
import { motion } from 'framer-motion'
import { Eye } from 'lucide-react'
import type { PatientRow } from '@/types/patient'
import { getPatientRowId, isHedisRow } from '@/types/patient'
import { formatUsDate } from '@/lib/formatDate'
import { formatPhoneDisplay } from '@/lib/formatPhone'
import { getRowApptDate, getRowCoverageEnds, getRowDos, getRowMed1, getRowPcpName, getRowRefillDue, getRowValue1, getRowValue2 } from '@/lib/patientRowValues'
import { filterDisplayedPatients } from '@/lib/patientTableFilter'
import { useStatusColorStore } from '@/stores/statusColorStore'
import { type SourceFilterState } from '@/types/filters'
import { resolveRowStatusStyle } from '@/types/statusColor'
import { RowDetailModal } from './RowDetailModal'
import { TablePagination } from './TablePagination'

interface DataTableProps {
  data: PatientRow[]
  globalFilter: string
  sourceFilter: SourceFilterState
  isLoading?: boolean
}

export function DataTable({
  data,
  globalFilter,
  sourceFilter,
  isLoading = false,
}: DataTableProps) {
  const [sorting, setSorting] = useState<SortingState>([])
  const [pagination, setPagination] = useState<PaginationState>({
    pageIndex: 0,
    pageSize: 15,
  })
  const [selectedRow, setSelectedRow] = useState<PatientRow | null>(null)
  const hedisStatus = useStatusColorStore((state) => state.hedisStatus)
  const loadStatusColors = useStatusColorStore((state) => state.loadStatusColors)
  const isStatusColorsLoaded = useStatusColorStore((state) => state.isLoaded)

  useEffect(() => {
    if (!isStatusColorsLoaded) {
      void loadStatusColors()
    }
  }, [isStatusColorsLoaded, loadStatusColors])

  const columns = useMemo<ColumnDef<PatientRow>[]>(
    () => [
      {
        header: 'Insurance',
        accessorKey: 'ins_name',
        cell: ({ row }) => row.original.ins_name || row.original.ins_id || '—',
      },
      {
        header: 'MID',
        accessorKey: 'pt_subno',
        cell: ({ row }) => row.original.pt_subno || '—',
      },
      {
        header: 'Patient Name',
        accessorFn: (row) => `${row.pt_fname} ${row.pt_lname}`,
        cell: ({ row }) => (
          <span className="cell-patient__name">
            {row.original.pt_fname} {row.original.pt_lname}
          </span>
        ),
      },
      {
        header: 'Quality Program',
        accessorKey: 'qp_name',
        cell: ({ row }) => row.original.qp_name || row.original.qp_id || '—',
      },
      {
        header: 'Phone',
        accessorKey: 'pt_phone',
        cell: ({ row }) => formatPhoneDisplay(row.original.pt_phone) || '—',
      },
      {
        header: 'DOB',
        accessorKey: 'pt_dob',
        cell: ({ getValue }) => formatUsDate(getValue() as string),
      },
      { header: 'Measure', accessorKey: 'measure' },
      {
        header: 'Appt Date',
        id: 'appt_date',
        accessorFn: (row) => getRowApptDate(row),
        cell: ({ row }) => getRowApptDate(row.original) || '—',
      },
      {
        header: 'PCP',
        id: 'pcp_name',
        accessorFn: (row) => getRowPcpName(row),
        cell: ({ row }) => getRowPcpName(row.original) || '—',
      },
      {
        header: 'Value 1',
        id: 'value1',
        accessorFn: (row) => getRowValue1(row),
        cell: ({ row }) =>
          isHedisRow(row.original) ? getRowValue1(row.original) || '—' : '—',
      },
      {
        header: 'Value 2',
        id: 'value2',
        accessorFn: (row) => getRowValue2(row),
        cell: ({ row }) =>
          isHedisRow(row.original) ? getRowValue2(row.original) || '—' : '—',
      },
      {
        header: 'DOS',
        accessorKey: 'dos',
        cell: ({ row }) => getRowDos(row.original) || '—',
      },
      {
        header: 'Medication',
        id: 'med_1',
        accessorFn: (row) => getRowMed1(row),
        cell: ({ row }) =>
          isHedisRow(row.original) ? '—' : getRowMed1(row.original) || '—',
      },
      {
        header: 'Refill Due',
        id: 'refill_due',
        accessorFn: (row) => getRowRefillDue(row),
        cell: ({ row }) =>
          isHedisRow(row.original) ? '—' : getRowRefillDue(row.original) || '—',
      },
      {
        header: 'COV END',
        id: 'coverage_ends',
        accessorFn: (row) => getRowCoverageEnds(row),
        cell: ({ row }) =>
          isHedisRow(row.original) ? '—' : getRowCoverageEnds(row.original) || '—',
      },
      {
        id: 'actions',
        header: '',
        cell: ({ row }) => (
          <button
            className="row-detail-btn"
            onClick={() => setSelectedRow(row.original)}
            title="View detailed information"
            aria-label={`View details for ${row.original.pt_fname} ${row.original.pt_lname}`}
          >
            <Eye size={18} />
          </button>
        ),
        size: 56,
      },
    ],
    [],
  )

  const tableData = useMemo(
    () => filterDisplayedPatients(data, sourceFilter, globalFilter),
    [data, sourceFilter, globalFilter],
  )

  const table = useReactTable({
    data: tableData,
    columns,
    getRowId: (row, index) => `${getPatientRowId(row)}::${index}`,
    state: { sorting, pagination },
    onSortingChange: setSorting,
    onPaginationChange: setPagination,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
  })

  const filteredRowCount = tableData.length
  const visibleRows = table.getRowModel().rows
  const emptyMessage =
    data.length === 0
      ? 'No records found.'
      : tableData.length === 0
        ? 'No records match the selected source.'
        : 'No records match your search.'

  useEffect(() => {
    setPagination((current) => ({ ...current, pageIndex: 0 }))
  }, [tableData, globalFilter])

  return (
    <>
      <motion.div
        className="data-table-wrap"
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.15 }}
      >
        <div className="data-table-scroll">
          <table className="data-table">
            <thead>
              {table.getHeaderGroups().map((hg) => (
                <tr key={hg.id}>
                  {hg.headers.map((header) => (
                    <th
                      key={header.id}
                      style={{ width: header.getSize() !== 150 ? header.getSize() : undefined }}
                      onClick={header.column.getToggleSortingHandler()}
                      className={header.column.getCanSort() ? 'sortable' : ''}
                    >
                      {flexRender(header.column.columnDef.header, header.getContext())}
                      {{
                        asc: ' ↑',
                        desc: ' ↓',
                      }[header.column.getIsSorted() as string] ?? null}
                    </th>
                  ))}
                </tr>
              ))}
            </thead>
            <tbody>
              {isLoading ? (
                <tr>
                  <td colSpan={columns.length} className="data-table__empty">
                    <div className="data-table__loading">
                      <div className="spinner" />
                      <p>Loading patient records...</p>
                    </div>
                  </td>
                </tr>
              ) : visibleRows.length === 0 ? (
                <tr>
                  <td colSpan={columns.length} className="data-table__empty">
                    {emptyMessage}
                  </td>
                </tr>
              ) : (
                visibleRows.map((row) => {
                  const statusStyle = resolveRowStatusStyle(hedisStatus, row.original.details)
                  const hasStatusColor = Boolean(
                    statusStyle.backgroundColor || statusStyle.color,
                  )
                  const rowStyle = hasStatusColor
                    ? ({
                        '--status-row-bg': statusStyle.backgroundColor,
                        '--status-row-fg': statusStyle.color,
                      } as CSSProperties)
                    : undefined
                  const cellStyle = hasStatusColor
                    ? {
                        backgroundColor: statusStyle.backgroundColor,
                        color: statusStyle.color,
                      }
                    : undefined

                  return (
                    <tr
                      key={`${row.id}-${row.index}`}
                      className={hasStatusColor ? 'data-table__row--status-colored' : undefined}
                      style={rowStyle}
                      title={statusStyle.title}
                    >
                      {row.getVisibleCells().map((cell) => (
                        <td key={cell.id} style={cellStyle}>
                          {flexRender(cell.column.columnDef.cell, cell.getContext())}
                        </td>
                      ))}
                    </tr>
                  )
                })
              )}
            </tbody>
          </table>
        </div>

        <TablePagination
          pageIndex={pagination.pageIndex}
          pageCount={table.getPageCount()}
          pageSize={pagination.pageSize}
          totalRows={filteredRowCount}
          onPageChange={(pageIndex) => table.setPageIndex(pageIndex)}
        />
      </motion.div>

      <RowDetailModal row={selectedRow} onClose={() => setSelectedRow(null)} />
    </>
  )
}
