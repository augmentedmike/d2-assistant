import { invoke } from '@tauri-apps/api/core'

export interface CharacterListing {
  path: string
  name: string
  class: string
  level: number
  file_size: number
  modified: number
  backup_warning: string | null
}

export interface CharacterBackupVersion {
  id: number
  created_at: number
  file_size: number
}

export interface CharacterData {
  name: string
  class: string
  level: number
  experience: number
  strength: number
  dexterity: number
  vitality: number
  energy: number
  life: number
  mana: number
  stamina: number
  gold: number
  stat_points: number
  skill_points: number
  file_size: number
  stats_validated: boolean
  equipped_items: EquippedItem[]
}

export interface EquippedItem {
  code: string
  name: string
  item_type: string
  slot: string
  identified: boolean
  ethereal: boolean
  socketed: boolean
}

export interface StashData {
  file_size: number
  item_count: number
  gold: number
}

export interface BankItem {
  id: number
  base_item: string
  affixes_json: string
  level_req: number
  class_req: string
  sockets: number
  icon_path: string | null
  source_character: string
  banked_at: number
  updated_at: number
  has_item_data: boolean
  transferred_to: string | null
}

export interface BankImportResult {
  items: BankItem[]
  backup_path: string | null
  source_cleared: boolean
  source_warning: string | null
}

export type NewBankItem = {
  base_item: string
  affixes_json: string
  level_req: number
  class_req: string
  sockets: number
  icon_path: string | null
  source_character: string
}

type ParseResult<T> = { type: 'Ok'; data: T } | { type: 'Err'; data: string }

function unwrapParseResult<T>(result: ParseResult<T>): T {
  if (result.type === 'Err') {
    throw new Error(result.data)
  }
  return result.data
}

export async function scanCharacters(
  dir: string,
  autoBackups: boolean,
): Promise<CharacterListing[]> {
  return unwrapParseResult(
    await invoke<ParseResult<CharacterListing[]>>('scan_characters', { dir, autoBackups }),
  )
}

export async function characterBackupHistory(characterPath: string): Promise<CharacterBackupVersion[]> {
  return await invoke<CharacterBackupVersion[]>('character_backup_history', { characterPath })
}

export async function characterRestoreBackup(characterPath: string, backupId: number): Promise<string> {
  return await invoke<string>('character_restore_backup', { characterPath, backupId })
}

export async function parseCharacter(path: string): Promise<CharacterData> {
  return unwrapParseResult(
    await invoke<ParseResult<CharacterData>>('parse_character', { path }),
  )
}

export async function parseStash(path: string): Promise<StashData> {
  return unwrapParseResult(
    await invoke<ParseResult<StashData>>('parse_stash', { path }),
  )
}

/** Bank commands return Tauri Result — Ok value resolves, Err rejects the promise. */
export async function bankAddItem(item: NewBankItem): Promise<BankItem> {
  return await invoke<BankItem>('bank_add_item', item)
}

export async function bankGetItems(): Promise<BankItem[]> {
  return await invoke<BankItem[]>('bank_get_items')
}

export async function bankDeleteItem(id: number): Promise<void> {
  await invoke<void>('bank_delete_item', { id })
}

/** Back up and move every complete item record from a selected D2X stash into the bank. */
export async function bankImportStashItems(stashPath: string): Promise<BankImportResult> {
  return await invoke<BankImportResult>('bank_import_stash_items', { stashPath })
}

export async function bankExportItems(
  itemIds: number[],
  characterPath: string,
  destination: 'inventory' | 'stash',
): Promise<string> {
  return await invoke<string>('bank_export_items', {
    itemIds,
    characterPath,
    destination,
  })
}
