import { useCallback, useEffect, useId, useRef, useState } from 'react'
import clsx from 'clsx'
import { Check, ChevronDown, Loader2 } from 'lucide-react'

export interface FilterMultiSelectOption {
  value: string
  label: string
}

interface FilterMultiSelectProps {
  values: string[]
  onChange: (values: string[]) => void
  options: FilterMultiSelectOption[]
  placeholder: string
  disabled?: boolean
  loading?: boolean
  ariaLabel: string
  className?: string
}

export function FilterMultiSelect({
  values,
  onChange,
  options,
  placeholder,
  disabled = false,
  loading = false,
  ariaLabel,
  className,
}: FilterMultiSelectProps) {
  const [open, setOpen] = useState(false)
  const rootRef = useRef<HTMLDivElement>(null)
  const listId = useId()

  const selectedOptions = options.filter((option) => values.includes(option.value))
  const displayLabel =
    selectedOptions.length === 0
      ? placeholder
      : selectedOptions.length === 1
        ? selectedOptions[0].label
        : `${selectedOptions.length} selected`

  const close = useCallback(() => setOpen(false), [])

  useEffect(() => {
    if (!open) return

    const handlePointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        close()
      }
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') close()
    }

    document.addEventListener('mousedown', handlePointerDown)
    document.addEventListener('keydown', handleKeyDown)
    return () => {
      document.removeEventListener('mousedown', handlePointerDown)
      document.removeEventListener('keydown', handleKeyDown)
    }
  }, [open, close])

  const toggleValue = (nextValue: string) => {
    if (values.includes(nextValue)) {
      onChange(values.filter((value) => value !== nextValue))
      return
    }
    onChange([...values, nextValue])
  }

  return (
    <div
      ref={rootRef}
      className={clsx('filter-select', open && 'filter-select--open', className)}
    >
      <button
        type="button"
        className="filter-select__trigger"
        onClick={() => !disabled && !loading && setOpen((prev) => !prev)}
        disabled={disabled || loading || options.length === 0}
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listId}
      >
        <span
          className={clsx(
            'filter-select__value',
            selectedOptions.length === 0 && 'filter-select__value--placeholder',
          )}
        >
          {loading ? 'Loading...' : displayLabel}
        </span>
        {loading ? (
          <Loader2 size={16} className="filter-select__chevron spin" />
        ) : (
          <ChevronDown size={16} className="filter-select__chevron" />
        )}
      </button>

      {open && (
        <ul id={listId} className="filter-select__menu" role="listbox" aria-label={ariaLabel}>
          {options.map((option) => {
            const isSelected = values.includes(option.value)
            return (
              <li key={option.value} role="presentation">
                <button
                  type="button"
                  role="option"
                  aria-selected={isSelected}
                  className={clsx(
                    'filter-select__option',
                    isSelected && 'filter-select__option--selected',
                  )}
                  onClick={() => toggleValue(option.value)}
                >
                  <span className="filter-select__option-label">{option.label}</span>
                  {isSelected && <Check size={16} className="filter-select__check" />}
                </button>
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}
