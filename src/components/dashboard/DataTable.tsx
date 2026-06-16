import { useEffect, useMemo, useState } from 'react'
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
import { Eye, Pill, Stethoscope } from 'lucide-react'
import type { PatientRow } from '@/types/patient'
import { getPatientRowId } from '@/types/patient'
import { formatUsDate } from '@/lib/formatDate'
import { getRowApptDate, getRowDos, getRowValue1, getRowValue2 } from '@/lib/patientRowValues'
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

function SourceBadge({ source }: { source: PatientRow['source'] }) {
  const isHedis = source === 'hedis'
  return (
    <span className={`type-badge type-badge--${source}`}>
      {isHedis ? <Stethoscope size={14} /> : <Pill size={14} />}
      {isHedis ? 'HEDIS' : 'Med Adh'}
    </span>
  )
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

  const columns = useMemo<ColumnDef<PatientRow>[]>(
    () => [
      {
        id: 'source',
        header: 'Source',
        accessorKey: 'source',
        cell: ({ getValue }) => (
          <SourceBadge source={getValue() as PatientRow['source']} />
        ),
        size: 110,
      },
      {
        header: 'Insurance',
        accessorKey: 'ins_name',
        cell: ({ row }) => row.original.ins_name || row.original.ins_id || '—',
      },
      {
        header: 'Quality Program',
        accessorKey: 'qp_name',
        cell: ({ row }) => row.original.qp_name || row.original.qp_id || '—',
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
        header: 'MID',
        accessorKey: 'pt_subno',
        cell: ({ row }) => row.original.pt_subno || '—',
      },
      { header: 'Phone', accessorKey: 'pt_phone' },
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
        header: 'Value 1',
        id: 'value1',
        accessorFn: (row) => getRowValue1(row),
        cell: ({ row }) => getRowValue1(row.original),
      },
      {
        header: 'Value 2',
        id: 'value2',
        accessorFn: (row) => getRowValue2(row),
        cell: ({ row }) => getRowValue2(row.original),
      },
      {
        header: 'DOS',
        accessorKey: 'dos',
        cell: ({ row }) => getRowDos(row.original) || '—',
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
                visibleRows.map((row, i) => {
                  const statusStyle = resolveRowStatusStyle(hedisStatus, row.original.details)
                  const hasStatusColor = Boolean(
                    statusStyle.backgroundColor || statusStyle.color,
                  )

                  return (
                  <motion.tr
                    key={`${row.id}-${row.index}`}
                    className={hasStatusColor ? 'data-table__row--status-colored' : undefined}
                    style={{
                      backgroundColor: statusStyle.backgroundColor,
                      color: statusStyle.color,
                    }}
                    title={statusStyle.title}
                    initial={{ opacity: 0, x: -8 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ delay: i * 0.03, duration: 0.25 }}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <td
                        key={cell.id}
                        style={statusStyle.color ? { color: statusStyle.color } : undefined}
                      >
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </td>
                    ))}
                  </motion.tr>
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
