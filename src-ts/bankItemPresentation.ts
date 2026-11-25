import type { BankItem, CharacterListing } from './lib/tauri'

export type Compatibility = 'usable' | 'unusable' | 'unknown'
export type BankTransferFilter = 'all' | 'available' | 'transferred'
export type BankCompatibilityFilter = 'all' | Compatibility

export type BankItemFilters = {
  search: string
  type: string
  class: string
  rarity: string
  transfer: BankTransferFilter
  compatibility: BankCompatibilityFilter
  minLevel: string
  maxLevel: string
  sockets: string
}

export type BankItemDetails = {
  type: string
  rarity: string
  properties: string[]
  attributes: { label: string; value: string }[]
}

function parseAffixes(json: string): string[] {
  try {
    const parsed: unknown = JSON.parse(json)
    return Array.isArray(parsed) ? parsed.map(String) : []
  } catch {
    return []
  }
}

export function bankItemDetails(item: BankItem): BankItemDetails {
  const values = parseAffixes(item.affixes_json)
  const type = values.find((value) => value.startsWith('Type: '))?.slice(6) ?? 'Uncategorized'
  const rarity = values.find((value) => value.startsWith('Rarity: '))?.slice(8) ?? 'Unknown'
  const metadataLabels = ['Type: ', 'D2 item code: ', 'Rarity: ']
  const properties = values.filter(
    (value) => !metadataLabels.some((label) => value.startsWith(label)) &&
      !['Identified', 'Unidentified', 'Ethereal', 'Non-ethereal', 'Original item record retained for transfer'].includes(value),
  )

  const attributes: BankItemDetails['attributes'] = []
  if (item.level_req < 0) {
    attributes.push({ label: 'Level req.', value: 'Unknown' })
  } else if (item.level_req > 0) {
    attributes.push({ label: 'Level req.', value: String(item.level_req) })
  }
  if (item.class_req && item.class_req !== 'Unknown') {
    attributes.push({ label: 'Class req.', value: item.class_req })
  }
  if (['Weapon', 'Armor', 'Shield'].includes(type) && item.sockets !== 0) {
    attributes.push({
      label: 'Sockets',
      value: item.sockets < 0 ? 'Socketed' : String(item.sockets),
    })
  }

  return { type, rarity, properties, attributes }
}

export function itemCompatibility(item: BankItem, character: CharacterListing | undefined): Compatibility {
  if (!character || !item.class_req || item.class_req === 'Unknown') return 'unknown'
  if (item.class_req !== 'Any' && item.class_req !== character.class) return 'unusable'
  if (item.level_req < 0) return 'unknown'
  if (character.level < item.level_req) return 'unusable'
  return 'usable'
}

function matchesSearch(item: BankItem, search: string): boolean {
  return [item.base_item, item.affixes_json, item.class_req]
    .join(' ')
    .toLowerCase()
    .includes(search)
}

function matchesTransfer(item: BankItem, filter: BankTransferFilter): boolean {
  if (filter === 'available') return !item.transferred_to
  if (filter === 'transferred') return Boolean(item.transferred_to)
  return true
}

function matchesCompatibility(
  item: BankItem,
  character: CharacterListing | undefined,
  filter: BankCompatibilityFilter,
): boolean {
  return filter === 'all' || itemCompatibility(item, character) === filter
}

function matchesLevelRange(item: BankItem, minimum: string, maximum: string): boolean {
  if (minimum && (item.level_req < 0 || item.level_req < Number(minimum))) return false
  if (maximum && (item.level_req < 0 || item.level_req > Number(maximum))) return false
  return true
}

export function filterBankItems(
  items: BankItem[],
  details: ReadonlyMap<number, BankItemDetails>,
  filters: BankItemFilters,
  character: CharacterListing | undefined,
): BankItem[] {
  const search = filters.search.trim().toLowerCase()
  return items.filter((item) => {
    const itemDetails = details.get(item.id) ?? bankItemDetails(item)
    if (search && !matchesSearch(item, search)) return false
    if (filters.type && itemDetails.type !== filters.type) return false
    if (filters.class && item.class_req !== filters.class) return false
    if (filters.rarity && itemDetails.rarity !== filters.rarity) return false
    if (!matchesTransfer(item, filters.transfer)) return false
    if (!matchesCompatibility(item, character, filters.compatibility)) return false
    if (!matchesLevelRange(item, filters.minLevel, filters.maxLevel)) return false
    if (filters.sockets && String(item.sockets) !== filters.sockets) return false
    return true
  })
}

export type GoldDraftValidation =
  | { valid: true; carriedGold: number; stashGold: number }
  | { valid: false; error: string }

export function validateGoldDrafts(
  level: number,
  carriedDraft: string,
  stashDraft: string,
): GoldDraftValidation {
  const carriedGold = Number(carriedDraft)
  const stashGold = Number(stashDraft)
  const carriedLimit = level * 10_000
  if (!Number.isInteger(carriedGold) || carriedGold < 0 || carriedGold > carriedLimit) {
    return {
      valid: false,
      error: `Carried gold must be a whole number from 0 to ${carriedLimit.toLocaleString()} at level ${level}.`,
    }
  }
  if (!Number.isInteger(stashGold) || stashGold < 0 || stashGold > 2_500_000) {
    return { valid: false, error: 'Stash gold must be a whole number from 0 to 2,500,000.' }
  }
  return { valid: true, carriedGold, stashGold }
}
