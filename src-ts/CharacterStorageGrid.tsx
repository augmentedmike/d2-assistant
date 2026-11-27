import { useState } from 'react'
import { createPortal } from 'react-dom'
import type { CharacterStorageItem } from './lib/tauri'
import { classicItemIcon } from './itemIcons'
import { formatStoredProperty } from './bankItemPresentation'

type Props = {
  title: string
  columns: number
  rows: number
  items: CharacterStorageItem[]
  selected: ReadonlySet<number>
  loading: boolean
  readOnly: boolean
  onToggle: (itemIndex: number) => void
  onMoveSelected: () => void
  onSelectAll: () => void
  onClear: () => void
}

export default function CharacterStorageGrid({
  title,
  columns,
  rows,
  items,
  selected,
  loading,
  readOnly,
  onToggle,
  onMoveSelected,
  onSelectAll,
  onClear,
}: Props) {
  const [hoverCard, setHoverCard] = useState<{ item: CharacterStorageItem; left: number; top: number } | null>(null)

  function showDetails(item: CharacterStorageItem, element: HTMLElement) {
    const rect = element.getBoundingClientRect()
    setHoverCard({
      item,
      left: Math.min(rect.right + 12, window.innerWidth - 336),
      top: Math.min(rect.top, window.innerHeight - 340),
    })
  }

  return (
    <section className="rounded-lg border border-neutral-800 bg-neutral-900 p-4">
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <div className="mr-auto">
          <h2 className="text-sm font-semibold uppercase tracking-wider text-gray-200">{title}</h2>
          <p className="mt-1 text-xs text-gray-500">{items.length} items · {selected.size} selected</p>
        </div>
        <button onClick={onSelectAll} disabled={items.length === 0 || loading} className="rounded border border-neutral-700 px-2 py-1.5 text-xs text-gray-300 hover:bg-neutral-800 disabled:opacity-40">Select all</button>
        <button onClick={onClear} disabled={selected.size === 0 || loading} className="rounded border border-neutral-700 px-2 py-1.5 text-xs text-gray-300 hover:bg-neutral-800 disabled:opacity-40">Clear</button>
        <button onClick={onMoveSelected} disabled={selected.size === 0 || loading || readOnly} title={readOnly ? 'Quit Diablo II before changing character saves' : undefined} className="rounded bg-red-800 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-700 disabled:opacity-40">Move selected to bank</button>
      </div>

      <div
        className="relative grid w-full overflow-hidden rounded border border-neutral-700 bg-neutral-950"
        style={{ width: `min(100%, ${columns * 32}px)`, gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`, gridTemplateRows: `repeat(${rows}, minmax(0, 1fr))`, aspectRatio: `${columns}/${rows}` }}
        aria-label={`${title} item grid`}
      >
        {Array.from({ length: columns * rows }, (_, index) => (
          <div key={index} className="border border-neutral-800/80" aria-hidden="true" />
        ))}
        {items.map((item) => {
          const isSelected = selected.has(item.item_index)
          const icon = classicItemIcon(item.code)
          return (
            <button
              key={item.item_index}
              type="button"
              aria-pressed={isSelected}
              aria-label={`${isSelected ? 'Deselect' : 'Select'} ${item.name}`}
              title={`${item.name} · ${item.code}`}
              onMouseEnter={(event) => showDetails(item, event.currentTarget)}
              onMouseLeave={() => setHoverCard(null)}
              onFocus={(event) => showDetails(item, event.currentTarget)}
              onBlur={() => setHoverCard(null)}
              onClick={() => onToggle(item.item_index)}
              className={`z-10 m-0.5 flex min-w-0 items-center justify-center overflow-hidden rounded border bg-neutral-900/90 p-0.5 transition-colors ${isSelected ? 'border-amber-300 ring-2 ring-amber-300/70' : 'border-neutral-700 hover:border-red-500 hover:bg-neutral-800'}`}
              style={{ gridColumn: `${item.x + 1} / span ${item.width}`, gridRow: `${item.y + 1} / span ${item.height}` }}
            >
              {icon ? (
                <img src={icon} alt="" className="h-full w-full object-contain drop-shadow-[0_1px_2px_rgba(0,0,0,0.9)]" draggable={false} />
              ) : (
                <span className="line-clamp-2 break-words text-center text-[10px] font-medium leading-tight text-gray-100">{item.name}</span>
              )}
            </button>
          )
        })}
        {items.length === 0 && <div className="pointer-events-none absolute inset-0 flex items-center justify-center text-xs text-gray-600">No items here</div>}
      </div>
      {hoverCard && createPortal(
        <aside
          role="tooltip"
          className="pointer-events-none fixed z-[1000] w-[min(320px,calc(100vw-24px))] max-h-[min(320px,calc(100vh-24px))] overflow-auto rounded-md border border-amber-700/70 bg-neutral-950/95 p-3 text-sm shadow-2xl shadow-black/70"
          style={{ left: Math.max(12, hoverCard.left), top: Math.max(12, hoverCard.top) }}
        >
          <div className="mb-2 border-b border-neutral-800 pb-2">
            <h3 className="font-semibold text-amber-100">{hoverCard.item.name}</h3>
            <p className="text-xs text-gray-500">Code: {hoverCard.item.code}</p>
          </div>
          <ul className="space-y-1 text-xs text-gray-200">
            {hoverCard.item.details.map((detail, index) => <li key={`${index}-${detail}`}>{formatStoredProperty(detail)}</li>)}
          </ul>
        </aside>,
        document.body,
      )}
    </section>
  )
}
