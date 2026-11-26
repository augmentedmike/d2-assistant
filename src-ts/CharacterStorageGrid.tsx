import type { CharacterStorageItem } from './lib/tauri'
import { classicItemIcon } from './itemIcons'

type Props = {
  title: string
  columns: number
  rows: number
  items: CharacterStorageItem[]
  selected: ReadonlySet<number>
  loading: boolean
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
  onToggle,
  onMoveSelected,
  onSelectAll,
  onClear,
}: Props) {
  return (
    <section className="rounded-lg border border-neutral-800 bg-neutral-900 p-4">
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <div className="mr-auto">
          <h2 className="text-sm font-semibold uppercase tracking-wider text-gray-200">{title}</h2>
          <p className="mt-1 text-xs text-gray-500">{items.length} items · {selected.size} selected</p>
        </div>
        <button onClick={onSelectAll} disabled={items.length === 0 || loading} className="rounded border border-neutral-700 px-2 py-1.5 text-xs text-gray-300 hover:bg-neutral-800 disabled:opacity-40">Select all</button>
        <button onClick={onClear} disabled={selected.size === 0 || loading} className="rounded border border-neutral-700 px-2 py-1.5 text-xs text-gray-300 hover:bg-neutral-800 disabled:opacity-40">Clear</button>
        <button onClick={onMoveSelected} disabled={selected.size === 0 || loading} className="rounded bg-red-800 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-700 disabled:opacity-40">Move selected to bank</button>
      </div>

      <div
        className="relative grid w-full overflow-hidden rounded border border-neutral-700 bg-neutral-950"
        style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`, gridTemplateRows: `repeat(${rows}, minmax(0, 1fr))`, aspectRatio: `${columns}/${rows}` }}
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
              onClick={() => onToggle(item.item_index)}
              className={`z-10 m-0.5 flex min-w-0 items-center justify-center overflow-hidden rounded border bg-neutral-900/90 p-0.5 transition-colors ${isSelected ? 'border-amber-300 ring-2 ring-amber-300/70' : 'border-neutral-700 hover:border-red-500 hover:bg-neutral-800'}`}
              style={{ gridColumn: `${item.x + 1} / span ${item.width}`, gridRow: `${item.y + 1} / span ${item.height}` }}
            >
              {icon ? (
                <img src={icon} alt="" className="d2-item-sprite h-full w-full object-contain drop-shadow-[0_1px_2px_rgba(0,0,0,0.9)]" draggable={false} />
              ) : (
                <span className="line-clamp-2 break-words text-center text-[10px] font-medium leading-tight text-gray-100">{item.name}</span>
              )}
            </button>
          )
        })}
        {items.length === 0 && <div className="pointer-events-none absolute inset-0 flex items-center justify-center text-xs text-gray-600">No items here</div>}
      </div>
    </section>
  )
}
