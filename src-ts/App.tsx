import { useState, useEffect, useCallback, useMemo, useRef } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import Runewords from './Runewords'
import CharacterBuildEditor from './CharacterBuildEditor'
import CharacterStorageGrid from './CharacterStorageGrid'
import { classicItemIcon } from './itemIcons'
import { bankItemDetails, filterBankItems, itemCompatibility, validateGoldDrafts } from './bankItemPresentation'
import type { Compatibility } from './bankItemPresentation'
import {
  scanCharacters,
  parseCharacter,
  characterStorageItems,
  bankGetItems,
  bankDeleteItem,
  bankImportStashItems,
  bankDepositCharacterItems,
  bankExportItems,
  characterBackupHistory,
  characterRestoreBackup,
  editCharacterGold,
  getAppSetting,
  setAppSetting,
} from './lib/tauri'
import type {
  CharacterListing,
  CharacterData,
  CharacterBackupVersion,
  BankItem,
  CharacterStorageItems,
} from './lib/tauri'

const CLASS_COLORS: Record<string, string> = {
  Amazon: 'text-sky-300',
  Sorceress: 'text-purple-400',
  Necromancer: 'text-green-400',
  Paladin: 'text-red-400',
  Barbarian: 'text-orange-400',
  Druid: 'text-yellow-400',
  Assassin: 'text-pink-400',
  Unknown: 'text-gray-400',
  Any: 'text-gray-400',
}

type Tab = 'characters' | 'bank' | 'runewords' | 'features'

function formatWhen(epochSecs: number): string {
  return new Date(epochSecs * 1000).toLocaleString()
}

export default function App() {
  const [tab, setTab] = useState<Tab>('characters')
  const [characterSubtab, setCharacterSubtab] = useState<'overview' | 'items' | 'skills' | 'backups'>('overview')
  const [saveDir, setSaveDir] = useState('')
  const [settingsLoaded, setSettingsLoaded] = useState(false)
  const [showExtendedRecipes, setShowExtendedRecipes] = useState(() => localStorage.getItem('d2-assistant.extended-recipes') === 'true')
  const [enableCharacterEditor, setEnableCharacterEditor] = useState(() => localStorage.getItem('d2-assistant.character-editor') !== 'false')
  const [autoCharacterBackups, setAutoCharacterBackups] = useState(() => localStorage.getItem('d2-assistant.auto-character-backups') !== 'false')
  const [characters, setCharacters] = useState<CharacterListing[]>([])
  const [selected, setSelected] = useState<CharacterListing | null>(null)
  const [characterData, setCharacterData] = useState<CharacterData | null>(null)
  const [characterStorage, setCharacterStorage] = useState<CharacterStorageItems>({ inventory: [], stash: [] })
  const [characterStorageSelection, setCharacterStorageSelection] = useState<{ inventory: Set<number>; stash: Set<number> }>({ inventory: new Set(), stash: new Set() })
  const [characterBackups, setCharacterBackups] = useState<CharacterBackupVersion[]>([])
  const [showAllCharacterBackups, setShowAllCharacterBackups] = useState(false)
  const [carriedGoldDraft, setCarriedGoldDraft] = useState('')
  const [stashGoldDraft, setStashGoldDraft] = useState('')
  const [editingGold, setEditingGold] = useState<'carried' | 'stash' | null>(null)
  const [bankItems, setBankItems] = useState<BankItem[]>([])
  const [selectedBankIds, setSelectedBankIds] = useState<Set<number>>(new Set())
  const [transferCharacterPath, setTransferCharacterPath] = useState('')
  const [transferDestination, setTransferDestination] = useState<'inventory' | 'stash'>('stash')
  const [notice, setNotice] = useState<string | null>(null)
  const [bankSearch, setBankSearch] = useState('')
  const [bankTypeFilter, setBankTypeFilter] = useState('')
  const [bankClassFilter, setBankClassFilter] = useState('')
  const [bankRarityFilter, setBankRarityFilter] = useState('')
  const [bankTransferFilter, setBankTransferFilter] = useState<'all' | 'available' | 'transferred'>('available')
  const [bankMatchFilter, setBankMatchFilter] = useState<'all' | Compatibility>('all')
  const [minLevelFilter, setMinLevelFilter] = useState('')
  const [maxLevelFilter, setMaxLevelFilter] = useState('')
  const [socketFilter, setSocketFilter] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const selectedCharacterPath = useRef<string | null>(null)

  const loadCharacters = useCallback(async () => {
    if (!settingsLoaded) return
    if (!saveDir.trim()) {
      setError('Choose your Diablo II save folder first.')
      return
    }
    setLoading(true)
    setError(null)
    try {
      await setAppSetting('save_directory', saveDir)
      const found = await scanCharacters(saveDir, autoCharacterBackups)
      setCharacters(found)
      setSelected((current) => current ? found.find((character) => character.path === current.path) ?? current : null)
      setTransferCharacterPath((current) => current || found[0]?.path || '')
      if (selectedCharacterPath.current) setCharacterBackups(await characterBackupHistory(selectedCharacterPath.current))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [saveDir, autoCharacterBackups, settingsLoaded])

  const monitorCharacterFolder = useCallback(async () => {
    if (!settingsLoaded || !saveDir.trim() || !autoCharacterBackups) return
    try {
      const found = await scanCharacters(saveDir, true)
      setCharacters(found)
      setSelected((current) => current ? found.find((character) => character.path === current.path) ?? current : null)
      if (selectedCharacterPath.current) {
        setCharacterBackups(await characterBackupHistory(selectedCharacterPath.current))
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [saveDir, autoCharacterBackups, settingsLoaded])

  const loadBank = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      setBankItems(await bankGetItems())
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    const timeout = window.setTimeout(() => void loadBank(), 0)
    return () => window.clearTimeout(timeout)
  }, [loadBank])

  useEffect(() => {
    let active = true
    const initializeSaveDirectory = async () => {
      try {
        const saved = await getAppSetting('save_directory')
        if (saved !== null) {
          if (active) setSaveDir(saved)
          localStorage.removeItem('d2-assistant.save-directory')
          return
        }
        // Migrate the previous WebView preference once into the persistent app database.
        const legacy = localStorage.getItem('d2-assistant.save-directory') ?? ''
        if (legacy.trim()) {
          await setAppSetting('save_directory', legacy)
          if (active) setSaveDir(legacy)
          localStorage.removeItem('d2-assistant.save-directory')
        }
      } catch (err) {
        if (active) {
          setError(err instanceof Error ? err.message : String(err))
          const legacy = localStorage.getItem('d2-assistant.save-directory') ?? ''
          if (legacy.trim()) setSaveDir(legacy)
        }
      } finally {
        if (active) setSettingsLoaded(true)
      }
    }
    void initializeSaveDirectory()
    return () => { active = false }
  }, [])

  useEffect(() => {
    if (!settingsLoaded || !saveDir.trim()) return
    const timeout = window.setTimeout(() => {
      void setAppSetting('save_directory', saveDir).catch((err: unknown) => {
        setError(err instanceof Error ? err.message : String(err))
      })
    }, 300)
    return () => window.clearTimeout(timeout)
  }, [saveDir, settingsLoaded])

  useEffect(() => {
    localStorage.setItem('d2-assistant.extended-recipes', String(showExtendedRecipes))
  }, [showExtendedRecipes])

  useEffect(() => {
    localStorage.setItem('d2-assistant.character-editor', String(enableCharacterEditor))
  }, [enableCharacterEditor])

  useEffect(() => {
    localStorage.setItem('d2-assistant.auto-character-backups', String(autoCharacterBackups))
  }, [autoCharacterBackups])

  useEffect(() => {
    if (!settingsLoaded || !saveDir.trim() || !autoCharacterBackups) return
    const interval = window.setInterval(() => void monitorCharacterFolder(), 30_000)
    return () => window.clearInterval(interval)
  }, [saveDir, autoCharacterBackups, settingsLoaded, monitorCharacterFolder])

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      if (!settingsLoaded) return
      if (tab === 'characters' && saveDir) void loadCharacters()
      if (tab === 'bank') {
        void loadBank()
        if (saveDir) void loadCharacters()
      }
    }, 0)
    return () => window.clearTimeout(timeout)
  }, [tab, saveDir, settingsLoaded, loadBank, loadCharacters])

  const handleSelect = async (char: CharacterListing) => {
    selectedCharacterPath.current = char.path
    setSelected(char)
    setCharacterSubtab('overview')
    setCharacterData(null)
    setCharacterBackups([])
    setCharacterStorage({ inventory: [], stash: [] })
    setCharacterStorageSelection({ inventory: new Set(), stash: new Set() })
    setLoading(true)
    setError(null)
    try {
      const [data, versions, storage] = await Promise.all([
        parseCharacter(char.path),
        characterBackupHistory(char.path),
        characterStorageItems(char.path),
      ])
      setCharacterData(data)
      setCharacterBackups(versions)
      setCharacterStorage(storage)
      setCarriedGoldDraft(String(data.gold))
      setStashGoldDraft(String(data.stash_gold))
      setShowAllCharacterBackups(false)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      setCharacterData(null)
      setCharacterBackups([])
      setCharacterStorage({ inventory: [], stash: [] })
      setCarriedGoldDraft('')
      setStashGoldDraft('')
    } finally {
      setLoading(false)
    }
  }

  const handleRestoreCharacterBackup = async (version: CharacterBackupVersion) => {
    if (!selected) return
    const confirmed = window.confirm(
      `Restore the character save from ${formatWhen(version.created_at)}? The current save will be backed up first. This replaces the selected .d2s file.`,
    )
    if (!confirmed) return
    setLoading(true)
    setError(null)
    try {
      const result = await characterRestoreBackup(selected.path, version.id)
      setNotice(result)
      await loadCharacters()
      await handleSelect(selected)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  const handleSaveGold = async (carriedDraft = carriedGoldDraft, stashDraft = stashGoldDraft) => {
    if (!selected || !characterData) return
    const gold = validateGoldDrafts(characterData.level, carriedDraft, stashDraft)
    if (!gold.valid) {
      setError(gold.error)
      return
    }
    const { carriedGold, stashGold } = gold
    if (carriedGold === characterData.gold && stashGold === characterData.stash_gold) return
    const confirmed = window.confirm(
      `Update gold in ${selected.path}?\n\nCarried: ${characterData.gold.toLocaleString()} → ${carriedGold.toLocaleString()}\nStash: ${characterData.stash_gold.toLocaleString()} → ${stashGold.toLocaleString()}\n\nA verified versioned backup is saved first.`,
    )
    if (!confirmed) return
    setLoading(true)
    setError(null)
    setNotice(null)
    try {
      const outputPath = await editCharacterGold(selected.path, carriedGold, stashGold)
      const refreshed = await scanCharacters(saveDir, autoCharacterBackups)
      setCharacters(refreshed)
      const outputCharacter = refreshed.find((character) => character.path === outputPath)
      if (outputCharacter) {
        await handleSelect(outputCharacter)
      } else {
        await handleSelect(selected)
      }
      setEditingGold(null)
      setNotice(`Saved gold changes to ${outputPath}. A verified versioned backup was created first.`)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  const chooseSaveDirectory = async () => {
    try {
      const selectedDirectory = await open({ directory: true, multiple: false })
      if (typeof selectedDirectory === 'string') setSaveDir(selectedDirectory)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  const handleDeleteBankItem = async (id: number) => {
    setError(null)
    try {
      await bankDeleteItem(id)
      setBankItems((prev) => prev.filter((item) => item.id !== id))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  const handleImportStashItems = async () => {
    setLoading(true)
    setError(null)
    setNotice(null)
    try {
      const selectedPath = await open({
        multiple: false,
        filters: [{ name: 'Diablo II stash', extensions: ['d2x'] }],
      })
      if (!selectedPath || Array.isArray(selectedPath)) return
      const result = await bankImportStashItems(selectedPath)
      setBankItems(await bankGetItems())
      setSelectedBankIds(new Set())
      setNotice(
        `Moved ${result.items.length} complete item records into the bank${result.backup_path ? '; verified backup created next to the stash' : ''}${result.source_cleared ? '; the selected stash is now empty' : result.source_warning ? `; source unchanged: ${result.source_warning}` : ''}.`,
      )
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  const handleDepositCharacterItems = async (source: 'stash' | 'inventory') => {
    if (!selected) return
    const sourceLabel = source === 'stash' ? 'private stash' : 'inventory'
    const selection = characterStorage[source].filter((item) => characterStorageSelection[source].has(item.item_index))
    if (selection.length === 0) return
    if (!window.confirm(`Move ${selection.length} selected item${selection.length === 1 ? '' : 's'} from ${selected.name}'s ${sourceLabel} to the shared bank? A verified versioned backup will be created first, then only these items will be removed from this character.`)) return
    setLoading(true)
    setError(null)
    setNotice(null)
    try {
      const count = await bankDepositCharacterItems(selected.path, source, selection.map(({ item_index, code, seed, fingerprint }) => ({ item_index, code, seed, fingerprint })))
      await Promise.all([loadBank(), handleSelect(selected)])
      setNotice(`Moved ${count} item${count === 1 ? '' : 's'} from ${selected.name}'s ${sourceLabel} to the shared bank. A verified character backup was created.`)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  const handleExportSelected = async () => {
    if (!transferCharacterPath) {
      setError('Scan for a character before transferring items.')
      return
    }
    const ids = [...selectedBankIds]
    if (ids.length === 0) return
    setLoading(true)
    setError(null)
    setNotice(null)
    try {
      const outputPath = await bankExportItems(ids, transferCharacterPath, transferDestination)
      setBankItems(await bankGetItems())
      setSelectedBankIds((previous) => {
        const next = new Set(previous)
        ids.forEach((id) => next.delete(id))
        return next
      })
      setNotice(`Transferred ${ids.length} item${ids.length === 1 ? '' : 's'} to ${outputPath}.`)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  const bankDetails = useMemo(
    () => new Map(bankItems.map((item) => [item.id, bankItemDetails(item)])),
    [bankItems],
  )
  const bankTypes = [...new Set(bankItems.map((item) => bankDetails.get(item.id)!.type))].sort()
  const bankClasses = [...new Set(bankItems.map((item) => item.class_req))].sort()
  const bankRarities = [...new Set(bankItems.map((item) => bankDetails.get(item.id)!.rarity))].sort()
  const matchCharacter = characters.find((character) => character.path === transferCharacterPath)
  const filteredBankItems = filterBankItems(bankItems, bankDetails, {
    search: bankSearch,
    type: bankTypeFilter,
    class: bankClassFilter,
    rarity: bankRarityFilter,
    transfer: bankTransferFilter,
    compatibility: bankMatchFilter,
    minLevel: minLevelFilter,
    maxLevel: maxLevelFilter,
    sockets: socketFilter,
  }, matchCharacter)
  const clearBankFilters = () => {
    setBankSearch('')
    setBankTypeFilter('')
    setBankClassFilter('')
    setBankRarityFilter('')
    setBankTransferFilter('available')
    setBankMatchFilter('all')
    setMinLevelFilter('')
    setMaxLevelFilter('')
    setSocketFilter('')
  }
  const selectableVisibleItems = filteredBankItems.filter(
    (item) => item.has_item_data && !item.transferred_to,
  )
  const selectVisibleBankItems = () => {
    setSelectedBankIds((previous) => {
      const next = new Set(previous)
      selectableVisibleItems.forEach((item) => next.add(item.id))
      return next
    })
  }
  const deselectVisibleBankItems = () => {
    setSelectedBankIds((previous) => {
      const next = new Set(previous)
      filteredBankItems.forEach((item) => next.delete(item.id))
      return next
    })
  }

  return (
    <div className="flex h-screen flex-col bg-neutral-950 text-white">
      <header className="flex shrink-0 flex-col border-b border-neutral-800 bg-neutral-900 px-4 py-2">
        <div className="flex min-h-10 flex-wrap items-center justify-between gap-x-4 gap-y-2">
          <div className="flex shrink-0 items-center gap-2"><img src="/brand.png" alt="" className="h-8 w-8 rounded-full object-cover" /><h1 className="flex-none whitespace-nowrap text-lg font-semibold tracking-wide">D2 Assistant</h1></div>
          <nav className="flex flex-wrap gap-1">
            {(
              [
                ['characters', 'Characters'],
                ['bank', 'Bank'],
                ['runewords', 'Recipes'],
                ['features', 'Features'],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                onClick={() => setTab(id)}
                className={`text-sm px-3 py-1 rounded ${
                  tab === id
                    ? 'bg-red-950/80 text-red-100 border border-red-800/70'
                    : 'text-gray-400 hover:text-white hover:bg-neutral-800'
                }`}
              >
                {label}
                {id === 'bank' && bankItems.length > 0 && tab !== 'bank' ? (
                  <span className="ml-1.5 text-xs text-gray-300">({bankItems.length})</span>
                ) : null}
              </button>
            ))}
          </nav>
        </div>
        {tab === 'characters' ? (
          <div className="mt-2 flex w-full min-w-0 items-center gap-2">
            <input
              type="text"
              placeholder={settingsLoaded ? 'Choose Diablo II save folder' : 'Restoring saved game location…'}
              value={saveDir}
              onChange={(e) => setSaveDir(e.target.value)}
              disabled={!settingsLoaded}
              className="min-w-0 flex-1 text-sm bg-neutral-800 border border-neutral-700 rounded px-2 py-1 text-white focus:outline-none focus:border-red-700"
            />
            <button onClick={() => void chooseSaveDirectory()} disabled={!settingsLoaded} className="shrink-0 text-sm bg-neutral-800 hover:bg-neutral-700 disabled:opacity-50 px-3 py-1 rounded">Browse…</button>
            <button
              onClick={() => void loadCharacters()}
              disabled={!settingsLoaded || loading || !saveDir}
              className="shrink-0 text-sm bg-red-800 hover:bg-red-700 disabled:opacity-50 px-3 py-1 rounded"
            >
              {loading ? 'Scanning...' : 'Scan'}
            </button>
          </div>
        ) : tab === 'bank' ? (
          <div className="mt-2 flex w-full items-center justify-end gap-2">
            <button
              onClick={() => void handleImportStashItems()}
              disabled={loading}
              className="text-sm bg-neutral-800 hover:bg-neutral-700 disabled:opacity-50 px-3 py-1 rounded"
            >
                  Choose stash to back up & move
            </button>
            <button
              onClick={() => void loadBank()}
              disabled={loading}
              className="text-sm bg-red-800 hover:bg-red-700 disabled:opacity-50 px-3 py-1 rounded"
            >
              {loading ? 'Loading...' : 'Refresh'}
            </button>
          </div>
        ) : null}
      </header>

      {error && (
        <div className="px-4 py-2 bg-red-900/30 border-b border-red-900/40 text-sm text-red-300">
          {error}
        </div>
      )}
      {notice && (
        <div className="px-4 py-2 bg-amber-950/30 border-b border-amber-900/50 text-sm text-amber-100 break-all">
          {notice}
        </div>
      )}

      {tab === 'characters' && (
        <div className="flex-1 overflow-hidden flex">
          <aside className="w-80 bg-neutral-900 border-r border-neutral-800 flex flex-col">
            <div className="p-4 border-b border-neutral-800">
              <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider">
                Characters
              </h2>
            </div>
            <div className="flex-1 overflow-y-auto">
              {characters.length === 0 && !loading && (
                <div className="p-4 text-xs text-gray-500 text-center">
                  {saveDir ? 'No characters found. Check the selected folder and scan again.' : 'Choose your Diablo II save folder above, then scan.'}
                </div>
              )}
              {loading && !selected && (
                <div className="p-4 text-xs text-gray-500 text-center animate-pulse">
                  Scanning...
                </div>
              )}
              <ul>
                {characters.map((char) => (
                  <li
                    key={char.path}
                    onClick={() => void handleSelect(char)}
                    className={`cursor-pointer px-4 py-3 border-b border-neutral-800 hover:bg-neutral-800/50 ${
                      selected?.path === char.path
                        ? 'bg-red-950/50 border-l-2 border-l-red-600'
                        : ''
                    }`}
                  >
                    <div className="flex justify-between items-start">
                      <div>
                        <div className="text-sm font-medium text-white text-left">
                          {char.name}
                        </div>
                        <div className="text-xs text-gray-400 text-left">
                          {char.class} • Lvl {char.level}
                        </div>
                      </div>
                      <div
                        className={`text-xs ${CLASS_COLORS[char.class] || CLASS_COLORS.Unknown}`}
                      >
                        L{char.level}
                      </div>
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          </aside>

          <main className="flex-1 overflow-y-auto bg-neutral-950 p-6 text-left">
            {!selected ? (
              <div className="text-center pt-20">
                <h2 className="text-xl text-gray-400">Select a character</h2>
                <p className="text-sm text-gray-500 mt-2">
                  Choose a character from the list to view their details.
                </p>
              </div>
            ) : (
              <div className="mx-auto max-w-6xl">
                <div className="flex justify-between items-start mb-6">
                  <div>
                    <h1 className="text-2xl font-bold text-white">{selected.name}</h1>
                    <div className="flex items-center gap-3 mt-1">
                      <span
                        className={`text-sm ${CLASS_COLORS[selected.class] || CLASS_COLORS.Unknown}`}
                      >
                        {selected.class}
                      </span>
                      <span className="text-sm text-gray-400">•</span>
                      <span className="text-sm text-gray-400">Level {selected.level}</span>
                      <span className="text-sm text-gray-400">•</span>
                      <span className="text-sm text-gray-400">
                        {(selected.file_size / 1024).toFixed(1)} KB
                      </span>
                    </div>
                  </div>
                  <button
                    onClick={() => void handleSelect(selected)}
                    className="text-sm bg-neutral-800 hover:bg-neutral-700 px-3 py-1 rounded text-white"
                  >
                    Refresh
                  </button>
                </div>

                <nav aria-label="Character sections" className="mb-6 flex gap-1 overflow-x-auto border-b border-neutral-800">
                  {([
                    ['overview', 'Overview'],
                    ['items', `Items · ${characterStorage.inventory.length + characterStorage.stash.length}`],
                    ['skills', 'Skills & stats'],
                    ['backups', `Backups · ${characterBackups.length}`],
                  ] as const).map(([key, label]) => (
                    <button key={key} onClick={() => setCharacterSubtab(key)} className={`shrink-0 border-b-2 px-4 py-2.5 text-sm font-medium transition-colors ${characterSubtab === key ? 'border-red-600 text-amber-100' : 'border-transparent text-gray-400 hover:text-white'}`} aria-current={characterSubtab === key ? 'page' : undefined}>
                      {label}
                    </button>
                  ))}
                </nav>

                {characterData ? (
                  <>
                  {characterSubtab === 'items' && <div className="mb-6 space-y-4">
                    <CharacterStorageGrid
                      title="Inventory (10 × 4)"
                      columns={10}
                      rows={4}
                      items={characterStorage.inventory}
                      selected={characterStorageSelection.inventory}
                      loading={loading}
                      onToggle={(itemIndex) => setCharacterStorageSelection((previous) => {
                        const inventory = new Set(previous.inventory)
                        if (inventory.has(itemIndex)) inventory.delete(itemIndex)
                        else inventory.add(itemIndex)
                        return { ...previous, inventory }
                      })}
                      onMoveSelected={() => void handleDepositCharacterItems('inventory')}
                      onSelectAll={() => setCharacterStorageSelection((previous) => ({ ...previous, inventory: new Set(characterStorage.inventory.map((item) => item.item_index)) }))}
                      onClear={() => setCharacterStorageSelection((previous) => ({ ...previous, inventory: new Set() }))}
                    />
                    <CharacterStorageGrid
                      title="Private stash (6 × 8)"
                      columns={6}
                      rows={8}
                      items={characterStorage.stash}
                      selected={characterStorageSelection.stash}
                      loading={loading}
                      onToggle={(itemIndex) => setCharacterStorageSelection((previous) => {
                        const stash = new Set(previous.stash)
                        if (stash.has(itemIndex)) stash.delete(itemIndex)
                        else stash.add(itemIndex)
                        return { ...previous, stash }
                      })}
                      onMoveSelected={() => void handleDepositCharacterItems('stash')}
                      onSelectAll={() => setCharacterStorageSelection((previous) => ({ ...previous, stash: new Set(characterStorage.stash.map((item) => item.item_index)) }))}
                      onClear={() => setCharacterStorageSelection((previous) => ({ ...previous, stash: new Set() }))}
                    />
                  </div>}
                  {characterSubtab === 'overview' && <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div className="bg-neutral-900 border border-neutral-800 rounded-lg p-4">
                      <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-3">
                        Attributes
                      </h2>
                      <div className="space-y-2 text-sm">
                        {(
                          [
                            ['Strength', characterData.strength],
                            ['Dexterity', characterData.dexterity],
                            ['Vitality', characterData.vitality],
                            ['Energy', characterData.energy],
                          ] as const
                        ).map(([label, value]) => (
                          <div key={label} className="flex justify-between">
                            <span className="text-gray-400">{label}</span>
                            <span className="text-white">{value}</span>
                          </div>
                        ))}
                      </div>
                      {!characterData.stats_validated && (
                        <div className="mt-3 p-2 bg-yellow-900/20 border border-yellow-900/30 rounded text-xs text-yellow-400">
                          This save’s stat block could not be validated. Zero values are placeholders.
                        </div>
                      )}
                    </div>

                    <div className="bg-neutral-900 border border-neutral-800 rounded-lg p-4">
                      <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-3">
                        Vitals
                      </h2>
                      <div className="space-y-2 text-sm">
                        {(
                          [
                            ['Life', characterData.life],
                            ['Mana', characterData.mana],
                            ['Stamina', characterData.stamina],
                          ] as const
                        ).map(([label, value]) => (
                          <div key={label} className="flex justify-between">
                            <span className="text-gray-400">{label}</span>
                            <span className="text-white">{value.toLocaleString()}</span>
                          </div>
                        ))}
                      </div>
                    </div>

                    <section className="rounded-lg border border-amber-900/60 bg-neutral-900 p-4">
                      <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-amber-200">Gold</h2>
                      <div className="space-y-4 text-sm">
                        {([
                          { key: 'carried', label: 'Belt / inventory', value: characterData.gold, limit: characterData.level * 10_000, draft: carriedGoldDraft, set: setCarriedGoldDraft },
                          { key: 'stash', label: 'Stash', value: characterData.stash_gold, limit: 2_500_000, draft: stashGoldDraft, set: setStashGoldDraft },
                        ] as const).map(({ key, label, value, limit, draft, set }) => (
                          <div key={key}>
                            <div className="flex items-center justify-between gap-3">
                              <span className="text-gray-400">{label}</span>
                              {editingGold === key ? (
                                <input
                                  autoFocus
                                  aria-label={`${label} gold`}
                                  type="number"
                                  min="0"
                                  max={limit}
                                  step="1"
                                  value={draft}
                                  onChange={(event) => {
                                    const raw = event.target.value
                                    const amount = Number(raw)
                                    set(raw === '' ? '' : String(Math.min(limit, Math.max(0, Number.isFinite(amount) ? Math.floor(amount) : 0))))
                                  }}
                                  onBlur={() => void handleSaveGold(key === 'carried' ? draft : carriedGoldDraft, key === 'stash' ? draft : stashGoldDraft)}
                                  onKeyDown={(event) => {
                                    if (event.key === 'Enter') event.currentTarget.blur()
                                    if (event.key === 'Escape') { set(key === 'carried' ? characterData.gold.toString() : characterData.stash_gold.toString()); setEditingGold(null) }
                                  }}
                                  disabled={loading || characterData.save_version !== 96 || !characterData.stats_validated}
                                  className="w-36 rounded border border-amber-700 bg-neutral-950 px-2 py-1 text-right text-white focus:outline-none"
                                />
                              ) : (
                                <button
                                  type="button"
                                  onClick={() => { setCarriedGoldDraft(String(characterData.gold)); setStashGoldDraft(String(characterData.stash_gold)); setEditingGold(key) }}
                                  disabled={loading || characterData.save_version !== 96 || !characterData.stats_validated}
                                  className="font-semibold text-white underline decoration-dotted decoration-amber-600 underline-offset-4 hover:text-amber-200 disabled:opacity-50"
                                >{value.toLocaleString()}</button>
                              )}
                            </div>
                            <div className="mt-1 flex items-center justify-between text-xs text-gray-500">
                              <span>Max {limit.toLocaleString()}</span>
                              <button type="button" className="text-amber-300 hover:text-amber-100 disabled:opacity-40" disabled={loading || value === limit || characterData.save_version !== 96 || !characterData.stats_validated} onClick={() => {
                                const carried = key === 'carried' ? String(limit) : String(characterData.gold)
                                const stash = key === 'stash' ? String(limit) : String(characterData.stash_gold)
                                setCarriedGoldDraft(carried); setStashGoldDraft(stash)
                                void handleSaveGold(carried, stash)
                              }}>Max out</button>
                            </div>
                          </div>
                        ))}
                      </div>
                      {characterData.save_version !== 96 && (
                        <p className="mt-3 rounded border border-amber-900/60 bg-amber-950/30 p-2 text-xs text-amber-200">Gold editing currently supports classic Lord of Destruction version 96 saves. This character is save version {characterData.save_version}.</p>
                      )}
                      {!characterData.stats_validated && (
                        <p className="mt-3 rounded border border-amber-900/60 bg-amber-950/30 p-2 text-xs text-amber-200">This character’s stat block could not be validated, so gold editing is disabled.</p>
                      )}
                      <div className="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-neutral-800 pt-4">
                        <p className="text-xs text-gray-400">Click either gold amount to edit. Changes save when you press Enter or leave the field.</p>
                      </div>
                    </section>

                    <div className="bg-neutral-900 border border-neutral-800 rounded-lg p-4">
                      <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-3">
                        Points
                      </h2>
                      <div className="space-y-2 text-sm">
                        <div className="flex justify-between">
                          <span className="text-gray-400">Stat Points</span>
                          <span className="text-white">{characterData.stat_points}</span>
                        </div>
                        <div className="flex justify-between">
                          <span className="text-gray-400">Skill Points</span>
                          <span className="text-white">{characterData.skill_points}</span>
                        </div>
                        <div className="flex justify-between">
                          <span className="text-gray-400">Experience</span>
                          <span className="text-white">
                            {characterData.experience.toLocaleString()}
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>}

                  {characterSubtab === 'skills' && <div className="space-y-4">
                    {!loading && enableCharacterEditor && <CharacterBuildEditor
                      key={selected.path}
                      selected={selected}
                      character={characterData}
                      onSaved={async (path) => {
                        const [refreshed, updatedData, versions] = await Promise.all([
                          scanCharacters(saveDir, autoCharacterBackups),
                          parseCharacter(path),
                          characterBackupHistory(path),
                        ])
                        setCharacters(refreshed)
                        const updatedCharacter = refreshed.find((item) => item.path === path)
                        if (updatedCharacter) setSelected(updatedCharacter)
                        setCharacterData(updatedData)
                        setCharacterBackups(versions)
                        setCarriedGoldDraft(String(updatedData.gold))
                        setStashGoldDraft(String(updatedData.stash_gold))
                        setNotice('Skill and attribute changes were saved with a verified backup.')
                      }}
                    />}
                    {!loading && !enableCharacterEditor && <section className="md:col-span-2 rounded-lg border border-neutral-800 bg-neutral-900 p-4"><h2 className="text-sm font-semibold uppercase tracking-wider text-amber-200">Skills & character points</h2><p className="mt-2 text-sm text-gray-400">Character editing is turned off. Enable it under Features to edit skill trees and attribute points.</p></section>}
                  </div>}

                  {characterSubtab === 'backups' && <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                    <div className="bg-neutral-900 border border-neutral-800 rounded-lg p-4">
                      <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider mb-3">
                        File
                      </h2>
                      <div className="space-y-2 text-sm text-gray-400">
                        <div className="flex justify-between gap-4">
                          <span>Path</span>
                          <span className="truncate max-w-xs">{selected.path}</span>
                        </div>
                        <div className="flex justify-between">
                          <span>Size</span>
                          <span>{characterData.file_size} bytes</span>
                        </div>
                      </div>
                    </div>

                    {selected.backup_warning && (
                      <div className="md:col-span-2 rounded border border-amber-900/60 bg-amber-950/30 p-3 text-xs text-amber-200">
                        Automatic backup could not be created: {selected.backup_warning}
                      </div>
                    )}

                    <section className="md:col-span-2 rounded-lg border border-neutral-800 bg-neutral-900 p-4">
                      <div className="mb-3 flex items-start justify-between gap-4">
                        <div>
                          <h2 className="text-sm font-semibold uppercase tracking-wider text-gray-300">Save history</h2>
                          <p className="mt-1 text-xs text-gray-500">Verified versions are stored locally. Restoring replaces this save after backing up its current version.</p>
                        </div>
                        <button onClick={() => void handleSelect(selected)} disabled={loading} className="rounded border border-neutral-700 px-3 py-1.5 text-xs text-gray-300 hover:bg-neutral-800 disabled:opacity-50">Refresh</button>
                      </div>
                      {characterBackups.length === 0 ? (
                        <p className="text-xs text-gray-500">No saved versions yet. Automatic backups are {autoCharacterBackups ? 'on' : 'off'}.</p>
                      ) : (
                        <ul className="space-y-2">
                          {(showAllCharacterBackups ? characterBackups : characterBackups.slice(0, 5)).map((version) => (
                            <li key={version.id} className="flex flex-wrap items-center justify-between gap-3 rounded border border-neutral-800 bg-neutral-950/60 px-3 py-2">
                              <div>
                                <div className="text-sm text-white">{formatWhen(version.created_at)}</div>
                                <div className="text-xs text-gray-500">{(version.file_size / 1024).toFixed(1)} KB · verified</div>
                              </div>
                              <button onClick={() => void handleRestoreCharacterBackup(version)} disabled={loading} className="rounded border border-amber-900/70 px-3 py-1.5 text-xs text-amber-200 hover:bg-amber-950/50 disabled:opacity-50">Restore this version</button>
                            </li>
                          ))}
                        </ul>
                      )}
                      {characterBackups.length > 5 && (
                        <button
                          onClick={() => setShowAllCharacterBackups((current) => !current)}
                          className="mt-3 text-xs font-medium text-amber-200 hover:text-amber-100"
                        >
                          {showAllCharacterBackups ? 'Show fewer versions' : `Show all ${characterBackups.length} versions`}
                        </button>
                      )}
                    </section>
                  </div>}

                    {characterSubtab === 'overview' && <section className="rounded-lg border border-neutral-800 bg-neutral-900 p-4">
                      <div className="flex items-start justify-between gap-4 mb-3">
                        <div>
                          <h2 className="text-sm font-semibold text-gray-300 uppercase tracking-wider">
                            Equipped items
                          </h2>
                          <p className="mt-1 text-xs text-gray-500">
                            Base names and slots are parsed; magical properties and item-specific requirements are still pending.
                          </p>
                        </div>
                        <span className="text-xs text-gray-400">
                          {characterData.equipped_items.length} found
                        </span>
                      </div>
                      {characterData.equipped_items.length === 0 ? (
                        <p className="text-sm text-gray-500">No equipped items were recognized in this save.</p>
                      ) : (
                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                          {characterData.equipped_items.map((item) => (
                            <div key={`${item.slot}-${item.code}`} className="flex items-start justify-between gap-3 rounded border border-neutral-800 bg-neutral-950/50 p-3">
                              <div className="flex min-w-0 items-center gap-3">
                                <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded border border-neutral-800 bg-black/40 p-1">
                                  {classicItemIcon(item.code) ? <img src={classicItemIcon(item.code)!} alt="" className="d2-item-sprite max-h-full max-w-full object-contain" draggable={false} /> : <span className="font-mono text-[9px] text-neutral-600">{item.code}</span>}
                                </div>
                                <div className="min-w-0">
                                  <div className="text-sm font-medium text-white">{item.name}</div>
                                  <div className="mt-1 text-xs text-gray-400">{item.slot} · {item.item_type} · {item.code}</div>
                                </div>
                              </div>
                              <div className="flex shrink-0 flex-wrap justify-end gap-1">
                                <span className="rounded bg-neutral-800 px-1.5 py-0.5 text-[10px] text-gray-300">
                                  {item.identified ? 'Identified' : 'Unidentified'}
                                </span>
                                {item.ethereal && <span className="rounded bg-neutral-800 px-1.5 py-0.5 text-[10px] text-neutral-300">Ethereal</span>}
                                {item.socketed && <span className="rounded bg-amber-950/50 px-1.5 py-0.5 text-[10px] text-amber-300">Socketed</span>}
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                    </section>}
                  </>
                ) : (
                  <div className="text-center pt-20">
                    <h2 className="text-xl text-gray-400">Loading character...</h2>
                  </div>
                )}
              </div>
            )}
          </main>
        </div>
      )}

      {tab === 'bank' && (
        <main className="flex-1 overflow-auto bg-neutral-950 p-6 text-left">
          <div className="mb-4">
            <h2 className="text-lg font-semibold text-white">Equipment Bank</h2>
            <p className="text-sm text-gray-400 mt-1">
              Showing {filteredBankItems.length} of {bankItems.length} item{bankItems.length === 1 ? '' : 's'}
            </p>
          </div>

            {bankItems.length > 0 && (
              <section className="mb-4 rounded-lg border border-red-950/70 bg-red-950/30 p-3">
                <div className="flex flex-wrap items-end gap-3">
                  <div className="mr-auto text-sm text-gray-300">
                    <span className="font-semibold text-white">{selectedBankIds.size} selected</span>
                    <span className="ml-2 text-xs text-gray-500">Selection stays while you change filters</span>
                  </div>
                  <button onClick={selectVisibleBankItems} disabled={selectableVisibleItems.length === 0} className="rounded border border-neutral-700 px-3 py-2 text-xs text-gray-200 hover:bg-neutral-800 disabled:opacity-40">
                    Select visible ({selectableVisibleItems.length})
                  </button>
                  <button onClick={deselectVisibleBankItems} disabled={!filteredBankItems.some((item) => selectedBankIds.has(item.id))} className="rounded border border-neutral-700 px-3 py-2 text-xs text-gray-300 hover:bg-neutral-800 disabled:opacity-40">
                    Deselect visible
                  </button>
                  <button onClick={() => setSelectedBankIds(new Set())} disabled={selectedBankIds.size === 0} className="rounded border border-neutral-700 px-3 py-2 text-xs text-gray-300 hover:bg-neutral-800 disabled:opacity-40">
                    Clear selection
                  </button>
                  <select value={transferCharacterPath} onChange={(e) => setTransferCharacterPath(e.target.value)} className="min-w-44 max-w-64 bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white">
                    {characters.length === 0 ? <option value="">No characters found</option> : characters.map((character) => <option key={character.path} value={character.path}>{character.name} · L{character.level}</option>)}
                  </select>
                  <select value={transferDestination} onChange={(e) => setTransferDestination(e.target.value as 'inventory' | 'stash')} className="bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white">
                    <option value="stash">Personal stash</option>
                    <option value="inventory">Inventory</option>
                  </select>
                  <button onClick={() => void handleExportSelected()} disabled={selectedBankIds.size === 0 || !transferCharacterPath || loading} className="rounded bg-red-800 px-3 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:opacity-40">
                    {loading ? 'Transferring…' : 'Transfer selected'}
                  </button>
                </div>
              </section>
            )}

          {bankItems.length > 0 && (
            <section className="mb-5 rounded-lg border border-neutral-800 bg-neutral-900/60 p-4">
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                <input
                  type="text"
                  inputMode="search"
                  autoComplete="off"
                  aria-label="Search bank items"
                  placeholder="Search names, codes, item details…"
                  value={bankSearch}
                  onChange={(e) => setBankSearch(e.target.value)}
                  className="bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white placeholder:text-gray-400 focus:outline-none focus:border-red-700"
                />
                <select value={bankTypeFilter} onChange={(e) => setBankTypeFilter(e.target.value)} className="bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white">
                  <option value="">All item types</option>
                  {bankTypes.map((type) => <option key={type} value={type}>{type}</option>)}
                </select>
                <select value={bankClassFilter} onChange={(e) => setBankClassFilter(e.target.value)} className="bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white">
                  <option value="">All class requirements</option>
                  {bankClasses.map((classReq) => <option key={classReq} value={classReq}>{classReq}</option>)}
                </select>
                <select value={bankRarityFilter} onChange={(e) => setBankRarityFilter(e.target.value)} className="bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white">
                  <option value="">All rarities</option>
                  {bankRarities.map((rarity) => <option key={rarity} value={rarity}>{rarity}</option>)}
                </select>
                <select value={bankTransferFilter} onChange={(e) => setBankTransferFilter(e.target.value as typeof bankTransferFilter)} className="bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white">
                  <option value="all">Include transfer history</option>
                  <option value="available">Available to transfer</option>
                  <option value="transferred">Already transferred</option>
                </select>
                <select value={bankMatchFilter} onChange={(e) => setBankMatchFilter(e.target.value as typeof bankMatchFilter)} className="bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white" disabled={!matchCharacter}>
                  <option value="all">All compatibility states</option>
                  <option value="usable">Usable by {matchCharacter?.name ?? 'character'}</option>
                  <option value="unusable">Does not meet requirements</option>
                  <option value="unknown">Requirements unknown</option>
                </select>
                <input
                  type="number"
                  min="0"
                  placeholder="Min level requirement"
                  value={minLevelFilter}
                  onChange={(e) => setMinLevelFilter(e.target.value)}
                  className="bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white placeholder:text-gray-400 focus:outline-none focus:border-red-700"
                />
                <input
                  type="number"
                  min="0"
                  placeholder="Max level requirement"
                  value={maxLevelFilter}
                  onChange={(e) => setMaxLevelFilter(e.target.value)}
                  className="bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white placeholder:text-gray-400 focus:outline-none focus:border-red-700"
                />
                <select value={socketFilter} onChange={(e) => setSocketFilter(e.target.value)} className="bg-neutral-800 border border-neutral-700 rounded px-3 py-2 text-sm text-white">
                  <option value="">Any socket count</option>
                  {[...new Set(bankItems.map((item) => item.sockets))].sort((a, b) => a - b).map((count) => <option key={count} value={count}>{count < 0 ? 'Socketed (count unknown)' : `${count} sockets`}</option>)}
                </select>
                <button onClick={clearBankFilters} className="rounded border border-neutral-700 px-3 py-2 text-sm text-gray-300 hover:bg-neutral-800">
                  Clear filters
                </button>
              </div>
            </section>
          )}

          {loading && bankItems.length === 0 ? (
            <div className="text-sm text-gray-500 animate-pulse">Loading bank...</div>
          ) : bankItems.length === 0 ? (
            <div className="rounded-lg border border-dashed border-neutral-800 p-10 text-center">
              <p className="text-gray-400">Bank is empty.</p>
              <p className="text-sm text-gray-500 mt-2">
                Choose a stash file. The app verifies a backup, moves its full item records into the bank, then empties the selected file.
              </p>
              <button
                onClick={() => void handleImportStashItems()}
                disabled={loading}
                className="mt-4 text-sm bg-red-800 hover:bg-red-700 disabled:opacity-50 px-3 py-1.5 rounded"
              >
                Choose stash to back up & move
              </button>
            </div>
          ) : filteredBankItems.length === 0 ? (
            <div className="rounded-lg border border-dashed border-neutral-800 p-10 text-center text-sm text-gray-400">
              No items match these filters.
              <button onClick={clearBankFilters} className="ml-2 text-amber-200 hover:underline">Clear filters</button>
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
              {filteredBankItems.map((item) => {
                const details = bankDetails.get(item.id)!
                const compatibility = itemCompatibility(item, matchCharacter)
                return (
                  <article key={item.id} className={`min-w-0 rounded-lg border p-4 shadow-sm transition-colors ${item.transferred_to ? 'border-neutral-800/70 bg-neutral-950/70 opacity-70' : 'border-neutral-800 bg-neutral-900/70 hover:border-red-900 hover:bg-neutral-900'}`}>
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex min-w-0 items-start gap-2">
                        <input
                          type="checkbox"
                          aria-label={`Select ${item.base_item}`}
                          checked={selectedBankIds.has(item.id)}
                          disabled={!item.has_item_data || Boolean(item.transferred_to)}
                          onChange={(event) => setSelectedBankIds((previous) => {
                            const next = new Set(previous)
                            if (event.target.checked) next.add(item.id)
                            else next.delete(item.id)
                            return next
                          })}
                          className="mt-1 accent-red-700 disabled:opacity-40"
                        />
                        <div className="flex h-16 w-16 shrink-0 items-center justify-center rounded-md border border-neutral-800 bg-black/40 p-1">
                          {details.iconPath ? (
                            <img src={details.iconPath} alt="" className="d2-item-sprite max-h-full max-w-full object-contain drop-shadow-[0_2px_4px_rgba(0,0,0,0.9)]" loading="lazy" draggable={false} />
                          ) : (
                            <span className="text-[9px] font-mono uppercase tracking-wide text-neutral-600">{item.base_item.slice(0, 4)}</span>
                          )}
                        </div>
                        <div className="min-w-0">
                        <h3 className="break-words font-semibold text-white">{item.base_item}</h3>
                        <p className="mt-1 text-[11px] uppercase tracking-wide text-amber-200">{details.type}</p>
                        {item.transferred_to && <span className="mt-2 inline-flex rounded border border-neutral-700 bg-neutral-800 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide text-gray-300">Archived transfer · unavailable</span>}
                        </div>
                      </div>
                      <span className="shrink-0 rounded-full border border-amber-400/20 bg-amber-400/10 px-2 py-0.5 text-[11px] text-amber-300">
                        {details.rarity}
                      </span>
                    </div>

                    {matchCharacter && <p className={`mt-2 text-[11px] ${compatibility === 'usable' ? 'text-amber-200' : compatibility === 'unusable' ? 'text-red-300' : 'text-gray-400'}`}>
                      {compatibility === 'usable' ? `Meets known requirements for ${matchCharacter.name}` : compatibility === 'unusable' ? `Does not meet requirements for ${matchCharacter.name}` : item.level_req < 0 ? 'Level requirement unknown — compatibility not guessed' : 'Class requirement unknown — compatibility not guessed'}
                    </p>}

                    {details.properties.length > 0 && (
                      <ul className="mt-3 space-y-1 border-t border-neutral-800 pt-3">
                        {details.properties.map((affix) => (
                          <li key={affix} className="text-xs leading-relaxed text-amber-200">{affix}</li>
                        ))}
                      </ul>
                    )}

                    {details.attributes.length > 0 && <dl className="mt-4 grid grid-cols-2 gap-x-3 gap-y-2 border-t border-neutral-800 pt-3 text-xs">
                      {details.attributes.map((attribute) => (
                        <div key={attribute.label}>
                          <dt className="text-gray-500">{attribute.label}</dt>
                          <dd className={`mt-0.5 ${attribute.label === 'Class req.' ? CLASS_COLORS[attribute.value] || CLASS_COLORS.Any : 'text-gray-200'}`}>{attribute.value}</dd>
                        </div>
                      ))}
                      <div className="min-w-0">
                      </div>
                    </dl>}

                    <footer className="mt-3 flex items-center justify-between gap-2 border-t border-neutral-800 pt-3">
                      <span className="truncate text-[11px] text-gray-500" title={item.transferred_to || formatWhen(item.banked_at)}>{item.transferred_to ? `Moved to ${item.transferred_to}` : item.has_item_data ? `Banked ${formatWhen(item.banked_at)}` : 'No game data'}</span>
                      <button
                        onClick={() => void handleDeleteBankItem(item.id)}
                        className="shrink-0 rounded px-2 py-1 text-xs text-red-400 hover:bg-red-400/10 hover:text-red-300"
                      >
                        Delete
                      </button>
                    </footer>
                  </article>
                )
              })}
            </div>
          )}
        </main>
      )}

      {tab === 'runewords' && <Runewords showExtendedRulesets={showExtendedRecipes} />}
      {tab === 'features' && (
        <main className="flex-1 overflow-y-auto bg-neutral-950 p-4 sm:p-6 text-left">
          <div className="mx-auto max-w-3xl">
            <h1 className="text-2xl font-bold text-white">Feature set</h1>
            <p className="mt-1 text-sm text-gray-400">Choose which optional game data and preview features appear in this installation. These preferences stay on this device.</p>
            <section className="mt-5 rounded-lg border border-neutral-800 bg-neutral-900/70 p-4">
              <label className="flex cursor-pointer items-start gap-3">
                <input type="checkbox" checked={showExtendedRecipes} onChange={(event) => setShowExtendedRecipes(event.target.checked)} className="mt-1 accent-red-700" />
                <span>
                  <span className="block text-sm font-semibold text-white">Show D2R and Reign of the Warlock recipe references</span>
                  <span className="mt-1 block text-xs leading-relaxed text-gray-400">Off by default. When enabled, the Recipes tab includes non-LoD runewords with game and expansion warnings. This does not add D2R save-file support.</span>
                </span>
              </label>
            </section>
            <section className="mt-4 rounded-lg border border-neutral-800 bg-neutral-900/70 p-4">
              <label className="flex cursor-pointer items-start gap-3">
                <input type="checkbox" checked={enableCharacterEditor} onChange={(event) => setEnableCharacterEditor(event.target.checked)} className="mt-1 accent-red-700" />
                <span>
                  <span className="block text-sm font-semibold text-white">Enable classic skill and attribute editing</span>
                  <span className="mt-1 block text-xs leading-relaxed text-gray-400">On by default because you requested it. Supports validated classic LoD version 96 saves. Changes always show a review and create a verified backup before writing.</span>
                </span>
              </label>
            </section>
            <section className="mt-4 rounded-lg border border-neutral-800 bg-neutral-900/70 p-4">
              <label className="flex cursor-pointer items-start gap-3">
                <input type="checkbox" checked={autoCharacterBackups} onChange={(event) => setAutoCharacterBackups(event.target.checked)} className="mt-1 accent-red-700" />
                <span>
                  <span className="block text-sm font-semibold text-white">Automatically back up changed character saves</span>
                  <span className="mt-1 block text-xs leading-relaxed text-gray-400">On by default. While D2 Assistant is open, it checks the selected folder every 30 seconds and saves one verified local version when a character save changes. You can restore a saved version from that character’s Save history. Restores always preserve the current version first.</span>
                </span>
              </label>
            </section>
          </div>
        </main>
      )}

    </div>
  )
}
