//! Bank persistence and verified transfers between stash, bank, and characters.

use crate::bank::BankItem;
use crate::d2x::{empty_d2x_stash, read_transferable_stash_items};
use crate::database::open_app_db;
use crate::grid::{find_grid_space, mark_grid};
use crate::{
    capture_character_backup, ensure_diablo_closed, now_secs, unique_sidecar_path,
    update_character_save_state,
};
use ::d2s::{
    BodyLocation, D2Save, Item, ItemMode, ItemPosition, StorePage, TxtData,
    bits::{BitReader, BitWriter},
    data::{ExternalData, GameTable},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::command;

const STASH_SOURCE: &str = "Stash import";

#[derive(Debug, Serialize)]
pub struct BankImportResult {
    items: Vec<BankItem>,
    backup_path: Option<String>,
    source_cleared: bool,
    source_warning: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CharacterStorageItem {
    item_index: usize,
    code: String,
    name: String,
    seed: u32,
    fingerprint: String,
    details: Vec<String>,
    x: u8,
    y: u8,
    width: u8,
    height: u8,
}

#[derive(Debug, Deserialize)]
pub struct CharacterItemSelection {
    item_index: usize,
    code: String,
    seed: u32,
    fingerprint: String,
}

#[derive(Debug, Serialize)]
pub struct CharacterStorageItems {
    inventory: Vec<CharacterStorageItem>,
    stash: Vec<CharacterStorageItem>,
}

#[derive(Debug, Serialize)]
pub struct BankDepositResult {
    moved: usize,
    duplicates: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BankDuplicateItem {
    id: i64,
    base_item: String,
    source_character: String,
    transferred_to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BankDuplicateGroup {
    code: String,
    seed: u32,
    items: Vec<BankDuplicateItem>,
}

#[command]
#[allow(clippy::too_many_arguments)] // Keep the existing flat Tauri invoke payload stable.
pub fn bank_add_item(
    app: tauri::AppHandle,
    base_item: String,
    affixes_json: String,
    level_req: i32,
    class_req: String,
    sockets: i32,
    icon_path: Option<String>,
    source_character: String,
) -> Result<BankItem, String> {
    let conn = open_app_db(&app)?;
    let now = now_secs()?;

    conn.execute(
        "INSERT INTO bank_items (base_item, affixes_json, level_req, class_req, sockets, icon_path, source_character, banked_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            base_item,
            affixes_json,
            level_req,
            class_req,
            sockets,
            icon_path,
            source_character,
            now,
            now
        ],
    )
    .map_err(|e| format!("insert error: {}", e))?;

    let id = conn.last_insert_rowid();
    Ok(BankItem {
        id,
        base_item,
        affixes_json,
        level_req,
        class_req,
        sockets,
        icon_path,
        source_character,
        banked_at: now,
        updated_at: now,
        has_item_data: false,
        transferred_to: None,
    })
}

#[command]
pub fn bank_get_items(app: tauri::AppHandle) -> Result<Vec<BankItem>, String> {
    let conn = open_app_db(&app)?;

    let mut stmt = conn
        .prepare(
            "SELECT id, base_item, affixes_json, level_req, class_req, sockets, icon_path, source_character, banked_at, updated_at,
                    item_data IS NOT NULL, transferred_to
             FROM bank_items
             ORDER BY banked_at DESC",
        )
        .map_err(|e| format!("prepare error: {}", e))?;

    let items: Vec<BankItem> = stmt
        .query_map([], |row| {
            Ok(BankItem {
                id: row.get(0)?,
                base_item: row.get(1)?,
                affixes_json: row.get(2)?,
                level_req: row.get(3)?,
                class_req: row.get(4)?,
                sockets: row.get(5)?,
                icon_path: row.get(6)?,
                source_character: row.get(7)?,
                banked_at: row.get(8)?,
                updated_at: row.get(9)?,
                has_item_data: row.get(10)?,
                transferred_to: row.get(11)?,
            })
        })
        .map_err(|e| format!("query error: {}", e))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("collect error: {}", e))?;

    Ok(items)
}

/// Find high-confidence duplicate fingerprints in the bank. Seed-zero items are omitted
/// because common gems and runes do not carry a useful unique fingerprint.
#[command]
pub fn bank_find_duplicates(app: tauri::AppHandle) -> Result<Vec<BankDuplicateGroup>, String> {
    let conn = open_app_db(&app)?;
    let mut statement = conn
        .prepare("SELECT id, base_item, source_character, transferred_to, item_data FROM bank_items WHERE item_data IS NOT NULL")
        .map_err(|e| format!("could not inspect bank item records: {e}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Vec<u8>>(4)?,
            ))
        })
        .map_err(|e| format!("could not read bank item records: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("could not collect bank item records: {e}"))?;
    let txt = TxtData::embedded();
    let mut groups = std::collections::HashMap::<(String, u32), Vec<BankDuplicateItem>>::new();
    for (id, base_item, source_character, transferred_to, item_data) in rows {
        let mut reader = BitReader::new(&item_data);
        let Ok(item) = Item::read(&mut reader, txt, 96) else {
            continue;
        };
        let code = item.code_string();
        if item.seed == 0 {
            continue;
        }
        groups
            .entry((code, item.seed))
            .or_default()
            .push(BankDuplicateItem {
                id,
                base_item,
                source_character,
                transferred_to,
            });
    }
    let mut duplicates = groups
        .into_iter()
        .filter(|(_, items)| items.len() > 1)
        .map(|((code, seed), items)| BankDuplicateGroup { code, seed, items })
        .collect::<Vec<_>>();
    duplicates.sort_by(|a, b| a.code.cmp(&b.code).then(a.seed.cmp(&b.seed)));
    Ok(duplicates)
}

#[command]
pub fn bank_delete_item(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    let conn = open_app_db(&app)?;
    let rows = conn
        .execute(
            "DELETE FROM bank_items WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| format!("delete error: {}", e))?;
    if rows == 0 {
        return Err(format!("item not found: {}", id));
    }
    Ok(())
}

/// Back up a selected stash, import every complete item record, then empty that stash.
#[command]
pub fn bank_import_stash_items(
    app: tauri::AppHandle,
    stash_path: String,
) -> Result<BankImportResult, String> {
    ensure_diablo_closed()?;
    let source_path = Path::new(&stash_path);
    let raw_stash = fs::read(source_path)
        .map_err(|e| format!("could not read stash {}: {e}", source_path.display()))?;
    let items = read_transferable_stash_items(&raw_stash)?;
    if items.is_empty() {
        return Err("stash is empty; the bank was left unchanged".into());
    }
    let now = now_secs()?;
    let backup = unique_sidecar_path(source_path, "bak", now);
    fs::copy(source_path, &backup).map_err(|e| format!("could not back up stash: {e}"))?;
    let backup_data =
        fs::read(&backup).map_err(|e| format!("could not verify stash backup: {e}"))?;
    if backup_data != raw_stash {
        return Err(format!(
            "stash backup verification failed: {}",
            backup.display()
        ));
    }
    let backup_path = Some(backup.to_string_lossy().to_string());

    let conn = open_app_db(&app)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("could not start bank import: {e}"))?;
    let txt = TxtData::embedded();
    let stat_names = txt
        .table(GameTable::ItemStatCost, 96)
        .map_err(|e| format!("could not load item stat names: {e}"))?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .skip(1)
        .map(|line| line.split('\t').next().unwrap_or("").to_string())
        .collect::<Vec<_>>();
    let mut imported = Vec::with_capacity(items.len());

    let mut occurrence_counts = std::collections::HashMap::<String, usize>::new();
    for parsed in items {
        let (base_name, item_type) = bank_base_details(&parsed.code, txt);
        let occurrence_key = format!("{}:{:08x}", parsed.code, parsed.seed);
        let occurrence = occurrence_counts.entry(occurrence_key).or_default();
        let source_key = format!("D2X:{}:{:08x}:{}", parsed.code, parsed.seed, *occurrence);
        *occurrence += 1;
        let mut metadata_values = vec![
            format!("Type: {item_type}"),
            format!("D2 item code: {}", parsed.code),
            format!("Rarity: {}", parsed.quality),
            if parsed.identified {
                "Identified".to_string()
            } else {
                "Unidentified".to_string()
            },
            if parsed.ethereal {
                "Ethereal".to_string()
            } else {
                "Non-ethereal".to_string()
            },
            "Original item record retained for transfer".to_string(),
        ];
        for (stat_id, value, layer) in parsed.stats {
            let name = stat_names
                .get(stat_id as usize)
                .map(String::as_str)
                .unwrap_or("unknown stat");
            let shift = txt
                .stat_info(::d2s::StatId(stat_id), 96)
                .map(|info| info.val_shift)
                .unwrap_or(0);
            let value = value >> shift;
            let layer = if layer == 0 {
                String::new()
            } else {
                format!(" (layer {layer})")
            };
            metadata_values.push(format!("{} {:+}{}", display_stat_name(name), value, layer));
        }
        let metadata = serde_json::to_string(&metadata_values)
            .map_err(|e| format!("could not encode item details: {e}"))?;
        let sockets = parsed.sockets;
        tx.execute(
            "INSERT INTO bank_items (base_item, affixes_json, level_req, class_req, sockets, icon_path, source_character, banked_at, updated_at, item_data, source_key, requirements_decoded)
             VALUES (?1, ?2, ?8, ?9, ?3, NULL, ?4, ?5, ?5, ?6, ?7, 1)
             ON CONFLICT(source_key) DO UPDATE SET base_item=excluded.base_item, affixes_json=excluded.affixes_json,
               level_req=excluded.level_req, class_req=excluded.class_req, sockets=excluded.sockets,
               item_data=excluded.item_data, updated_at=excluded.updated_at",
            rusqlite::params![base_name, metadata, sockets, STASH_SOURCE, now, parsed.item_data, source_key, parsed.level_req, parsed.class_req],
        ).map_err(|e| format!("could not import {}: {e}", parsed.code))?;
        let (id, transferred_to): (i64, Option<String>) = tx
            .query_row(
                "SELECT id, transferred_to FROM bank_items WHERE source_key = ?1",
                [&source_key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| format!("could not retrieve imported item: {e}"))?;
        imported.push(BankItem {
            id,
            base_item: base_name,
            affixes_json: metadata,
            level_req: parsed.level_req,
            class_req: parsed.class_req,
            sockets,
            icon_path: None,
            source_character: STASH_SOURCE.into(),
            banked_at: now,
            updated_at: now,
            has_item_data: true,
            transferred_to,
        });
    }
    ensure_diablo_closed()?;
    tx.commit()
        .map_err(|e| format!("could not finish stash import: {e}"))?;
    let empty = empty_d2x_stash();
    let temp_path = unique_sidecar_path(source_path, "empty-tmp", now);
    let result = (|| -> Result<(), String> {
        fs::write(&temp_path, &empty).map_err(|e| format!("could not prepare empty stash: {e}"))?;
        let check = read_transferable_stash_items(
            &fs::read(&temp_path).map_err(|e| format!("could not verify empty stash: {e}"))?,
        )?;
        if !check.is_empty() {
            return Err("empty stash verification found item records".into());
        }
        fs::rename(&temp_path, source_path)
            .map_err(|e| format!("could not clear source stash: {e}"))?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    let (source_cleared, source_warning) = (true, None);
    Ok(BankImportResult {
        items: imported,
        backup_path,
        source_cleared,
        source_warning,
    })
}

/// Read the actual item boxes in a character's carried inventory and private stash.
#[command]
pub fn character_storage_items(character_path: String) -> Result<CharacterStorageItems, String> {
    let bytes = fs::read(&character_path)
        .map_err(|e| format!("could not read character save {character_path}: {e}"))?;
    let save =
        D2Save::from_bytes(&bytes).map_err(|e| format!("could not decode character save: {e}"))?;
    let txt = TxtData::embedded();
    let stat_names = txt
        .table(GameTable::ItemStatCost, save.version)
        .map_err(|e| format!("could not load item stat names: {e}"))?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .skip(1)
        .map(|line| line.split('\t').next().unwrap_or("").to_string())
        .collect::<Vec<_>>();
    let mut inventory = Vec::new();
    let mut stash = Vec::new();
    for (item_index, item) in save.items.0.iter().enumerate() {
        if item.position.mode != ItemMode::Stored
            || !matches!(
                item.position.store_page,
                StorePage::Inventory | StorePage::Stash
            )
        {
            continue;
        }
        let info = txt.item_info(item.code, save.version).map_err(|e| {
            format!(
                "could not size {} in character storage: {e}",
                item.code_string()
            )
        })?;
        let (name, item_type) = bank_base_details(&item.code_string(), txt);
        let mut details = vec![item_type, format!("{:?} quality", item.quality)];
        let level_req = crate::d2x::item_level_requirement(
            &item.code_string(),
            &item.quality,
            item.flags.contains(::d2s::ItemFlags::RUNEWORD),
            txt,
        );
        details.push(if level_req < 0 {
            "Level requirement: Unknown".into()
        } else {
            format!("Level requirement: {level_req}")
        });
        details.push(format!(
            "Class requirement: {}",
            crate::d2x::item_class_requirement(&item.code_string(), txt)
        ));
        if item.flags.contains(::d2s::ItemFlags::IDENTIFIED) {
            details.push("Identified".into());
        } else {
            details.push("Unidentified".into());
        }
        if item.flags.contains(::d2s::ItemFlags::ETHEREAL) {
            details.push("Ethereal".into());
        }
        if item.level > 0 {
            details.push(format!("Item level {}", item.level));
        }
        if item.flags.contains(::d2s::ItemFlags::SOCKETED) || !item.sockets.is_empty() {
            details.push(format!("{} socket(s)", item.sockets.len()));
            for socketed in item.sockets.iter().flatten() {
                let (socket_name, _) = bank_base_details(&socketed.code_string(), txt);
                details.push(format!("Socketed: {socket_name}"));
            }
        }
        for stat in item
            .stats
            .iter()
            .chain(item.runeword_stats.iter().flatten())
            .chain(item.set_bonus_stats.iter().flatten())
        {
            let raw_name = stat_names
                .get(stat.id.0 as usize)
                .map(String::as_str)
                .unwrap_or("");
            if raw_name.is_empty() {
                continue;
            }
            let shift = txt
                .stat_info(stat.id, save.version)
                .map(|info| info.val_shift)
                .unwrap_or(0);
            let value = stat.value >> shift;
            let layer = if stat.layer == 0 {
                String::new()
            } else {
                format!(" (layer {})", stat.layer)
            };
            details.push(format!(
                "{} {:+}{}",
                display_stat_name(raw_name),
                value,
                layer
            ));
        }
        if item.flags.contains(::d2s::ItemFlags::RUNEWORD) {
            details.push("Runeword".into());
        }
        let storage_item = CharacterStorageItem {
            item_index,
            code: item.code_string(),
            name,
            seed: item.seed,
            fingerprint: item_selection_fingerprint(item, txt, save.version)?,
            details,
            x: item.position.inv_x,
            y: item.position.inv_y,
            width: info.inv_width.unwrap_or(1),
            height: info.inv_height.unwrap_or(1),
        };
        match item.position.store_page {
            StorePage::Inventory
                if storage_item.x + storage_item.width <= 10
                    && storage_item.y + storage_item.height <= 4 =>
            {
                inventory.push(storage_item)
            }
            StorePage::Stash
                if storage_item.x + storage_item.width <= 6
                    && storage_item.y + storage_item.height <= 8 =>
            {
                stash.push(storage_item)
            }
            StorePage::Inventory | StorePage::Stash => {
                return Err(format!(
                    "{} has an invalid saved storage position; no deposit is available",
                    item.code_string()
                ));
            }
            _ => unreachable!(),
        }
    }
    Ok(CharacterStorageItems { inventory, stash })
}

/// Move selected complete item records from a character's private stash or inventory to the bank.
#[command]
pub fn bank_deposit_character_items(
    app: tauri::AppHandle,
    character_path: String,
    source: String,
    selected_items: Vec<CharacterItemSelection>,
) -> Result<BankDepositResult, String> {
    ensure_diablo_closed()?;
    let store_page = match source.as_str() {
        "stash" => StorePage::Stash,
        "inventory" => StorePage::Inventory,
        _ => return Err("source must be stash or inventory".into()),
    };
    if selected_items.is_empty() {
        return Err("select at least one character item to move".into());
    }
    let selected_count = selected_items.len();
    let selected_by_index: std::collections::HashMap<_, _> = selected_items
        .into_iter()
        .map(|selection| {
            (
                selection.item_index,
                (selection.code, selection.seed, selection.fingerprint),
            )
        })
        .collect();
    if selected_by_index.len() != selected_count {
        return Err("the selected character item list contains duplicate indexes".into());
    }
    let path = PathBuf::from(&character_path);
    let original_bytes = fs::read(&path)
        .map_err(|e| format!("could not read character save {}: {e}", path.display()))?;
    let mut save = D2Save::from_bytes(&original_bytes)
        .map_err(|e| format!("could not decode character save: {e}"))?;
    if save.version != 96 {
        return Err(format!(
            "character save version {} is not supported for item deposits",
            save.version
        ));
    }
    let original_item_count = save.items.0.len();
    let txt = TxtData::embedded();
    let mut records = Vec::new();
    let mut remaining = Vec::with_capacity(original_item_count);
    for (item_index, item) in std::mem::take(&mut save.items.0).into_iter().enumerate() {
        if let Some((expected_code, expected_seed, expected_fingerprint)) =
            selected_by_index.get(&item_index)
        {
            if item.position.mode != ItemMode::Stored || item.position.store_page != store_page {
                return Err("selected item no longer occupies the requested storage page; refresh the character view".into());
            }
            if item.code_string() != *expected_code
                || item.seed != *expected_seed
                || item_selection_fingerprint(&item, txt, save.version)? != *expected_fingerprint
            {
                return Err(
                    "a selected item changed since the grid was loaded; refresh the character view"
                        .into(),
                );
            }
            let mut item_data = Vec::new();
            item.write(&mut BitWriter::new(&mut item_data), txt, save.version)
                .map_err(|e| {
                    format!("could not encode {} for the bank: {e}", item.code_string())
                })?;
            let (level_req, class_req) = crate::d2x::requirements_from_item_record(&item_data)?;
            records.push((item_index, item, item_data, level_req, class_req));
        } else {
            remaining.push((item_index, item));
        }
    }
    if records.len() != selected_by_index.len() {
        return Err(
            "some selected items no longer exist in the character save; refresh the character view"
                .into(),
        );
    }
    // Detect duplicates individually before creating a backup. A duplicate selection
    // stays in the character; unique selected items can still move in the same batch.
    let check_conn = open_app_db(&app)?;
    let bank_records = {
        let mut statement = check_conn
            .prepare("SELECT item_data, transferred_to FROM bank_items WHERE item_data IS NOT NULL")
            .map_err(|e| format!("could not inspect bank fingerprints: {e}"))?;
        statement
            .query_map([], |row| {
                Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(|e| format!("could not read bank fingerprints: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("could not read bank fingerprints: {e}"))?
    };
    let mut seen = std::collections::HashSet::new();
    let mut duplicates = Vec::new();
    let mut accepted = Vec::with_capacity(records.len());
    for record in records {
        let (item_index, item, _, _, _) = &record;
        let code = item.code_string();
        let key = (code.clone(), item.seed);
        let repeated_in_selection = item.seed != 0 && !seen.insert(key);
        let conflicting_duplicate = item.seed != 0
            && bank_records.iter().any(|(data, transferred_to)| {
                let mut reader = BitReader::new(data);
                Item::read(&mut reader, txt, save.version).is_ok_and(|bank_item| {
                    bank_item.code_string() == code
                        && bank_item.seed == item.seed
                        && transferred_to.as_deref() != Some(character_path.as_str())
                })
            });
        if repeated_in_selection || conflicting_duplicate {
            duplicates.push(format!("{} (fingerprint {:08x})", code, item.seed));
            remaining.push((*item_index, item.clone()));
        } else {
            accepted.push(record);
        }
    }
    let records = accepted;
    drop(check_conn);

    if records.is_empty() {
        save.items.0 = remaining.into_iter().map(|(_, item)| item).collect();
        return Ok(BankDepositResult {
            moved: 0,
            duplicates,
        });
    }

    // Do not change the save until its versioned backup is verified.
    capture_character_backup(&app, &path, true)?
        .ok_or_else(|| "could not create a verified character backup".to_string())?;
    let conn = open_app_db(&app)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("could not start character deposit: {e}"))?;
    let stat_names = txt
        .table(GameTable::ItemStatCost, save.version)
        .map_err(|e| format!("could not load item stat names: {e}"))?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .skip(1)
        .map(|line| line.split('\t').next().unwrap_or("").to_string())
        .collect::<Vec<_>>();
    let existing = {
        let mut statement = tx
            .prepare(
                "SELECT id, item_data, transferred_to FROM bank_items WHERE item_data IS NOT NULL",
            )
            .map_err(|e| format!("could not inspect bank fingerprints: {e}"))?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|e| format!("could not read bank fingerprints: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("could not read bank fingerprints: {e}"))?
    };
    let now = now_secs()?;
    let snapshot_hash = format!("{:x}", Sha256::digest(&original_bytes));
    let source_character = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Character");
    for (item_index, item, item_data, level_req, class_req) in &records {
        let code = item.code_string();
        let duplicate = (item.seed != 0)
            .then(|| {
                existing.iter().find_map(|(id, data, transferred_to)| {
                    let mut reader = BitReader::new(data);
                    let bank_item = Item::read(&mut reader, txt, save.version).ok()?;
                    (bank_item.code_string() == code && bank_item.seed == item.seed)
                        .then_some((*id, transferred_to.as_deref()))
                })
            })
            .flatten();
        if let Some((id, Some(target))) = duplicate
            && target == character_path
        {
            tx.execute(
                "UPDATE bank_items SET transferred_to=NULL, source_character=?1, banked_at=?2, updated_at=?2, item_data=?3, requirements_decoded=1 WHERE id=?4",
                rusqlite::params![source_character, now, item_data, id],
            ).map_err(|e| format!("could not return {} to the bank: {e}", code))?;
            continue;
        }
        if duplicate.is_some() {
            return Err(format!(
                "{} already exists in the bank by item fingerprint; character left unchanged",
                code
            ));
        }

        let (base_name, item_type) = bank_base_details(&code, txt);
        let mut values = vec![
            format!("Type: {item_type}"),
            format!("D2 item code: {code}"),
            format!("Rarity: {:?}", item.quality),
            if item.flags.contains(d2s::ItemFlags::IDENTIFIED) {
                "Identified"
            } else {
                "Unidentified"
            }
            .to_string(),
            if item.flags.contains(d2s::ItemFlags::ETHEREAL) {
                "Ethereal"
            } else {
                "Non-ethereal"
            }
            .to_string(),
            "Original item record retained for transfer".into(),
        ];
        for stat in &item.stats {
            let name = stat_names
                .get(stat.id.0 as usize)
                .map(String::as_str)
                .unwrap_or("unknown stat");
            let shift = txt
                .stat_info(stat.id, save.version)
                .map(|info| info.val_shift)
                .unwrap_or(0);
            let layer = if stat.layer == 0 {
                String::new()
            } else {
                format!(" (layer {})", stat.layer)
            };
            values.push(format!(
                "{} {:+}{}",
                display_stat_name(name),
                stat.value >> shift,
                layer
            ));
        }
        let metadata = serde_json::to_string(&values)
            .map_err(|e| format!("could not encode {} details: {e}", code))?;
        let path_hash = format!("{:x}", Sha256::digest(path.to_string_lossy().as_bytes()));
        let source_key = format!("D2S:{path_hash}:{snapshot_hash}:{item_index}");
        tx.execute(
            "INSERT INTO bank_items (base_item, affixes_json, level_req, class_req, sockets, icon_path, source_character, banked_at, updated_at, item_data, source_key, requirements_decoded)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?7, ?8, ?9, 1)",
            rusqlite::params![base_name, metadata, level_req, class_req, item.sockets.len() as i32, source_character, now, item_data, source_key],
        ).map_err(|e| format!("could not add {} to the bank: {e}", code))?;
    }

    remaining.sort_by_key(|(index, _)| *index);
    save.items.0 = remaining.into_iter().map(|(_, item)| item).collect();
    let output_bytes = save
        .to_bytes()
        .map_err(|e| format!("could not encode updated character: {e}"))?;
    let verified = D2Save::from_bytes(&output_bytes)
        .map_err(|e| format!("updated character failed validation: {e}"))?;
    if verified.items.len() + records.len() != original_item_count {
        return Err(
            "updated character failed item-count verification; source left unchanged".into(),
        );
    }
    let temp_path = unique_sidecar_path(&path, "deposit-tmp", now);
    fs::write(&temp_path, &output_bytes)
        .map_err(|e| format!("could not stage updated character: {e}"))?;
    if fs::read(&temp_path).ok() != Some(output_bytes.clone()) {
        let _ = fs::remove_file(&temp_path);
        return Err("updated character byte verification failed".into());
    }
    ensure_diablo_closed()?;
    fs::rename(&temp_path, &path).map_err(|e| format!("could not save updated character: {e}"))?;
    if let Err(error) = tx.commit() {
        let restore_path = unique_sidecar_path(&path, "deposit-restore", now);
        let restore_result = (|| -> std::io::Result<()> {
            fs::write(&restore_path, &original_bytes)?;
            fs::rename(&restore_path, &path)
        })();
        return Err(format!(
            "could not commit bank deposit: {error}{}",
            restore_result
                .err()
                .map(|e| format!("; restoring the original character also failed: {e}"))
                .unwrap_or_default()
        ));
    }
    update_character_save_state(&app, &path, &output_bytes)?;
    Ok(BankDepositResult {
        moved: records.len(),
        duplicates,
    })
}

fn item_selection_fingerprint(item: &Item, txt: &TxtData, version: u32) -> Result<String, String> {
    let mut normalized = item.clone();
    normalized.position = ItemPosition::default();
    let mut bytes = Vec::new();
    normalized
        .write(&mut BitWriter::new(&mut bytes), txt, version)
        .map_err(|error| format!("could not fingerprint {}: {error}", item.code_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn bank_base_details(code: &str, txt: &TxtData) -> (String, String) {
    let item_info = txt.item_info(::d2s::ItemCode::new(code), 96).ok();
    let category = if item_info.is_some_and(|info| info.is_weapon) {
        Some("Weapon")
    } else if item_info.is_some_and(|info| info.is_armor) {
        Some("Armor")
    } else if item_info.is_some_and(|info| info.is_charm) {
        Some("Charm")
    } else if crate::items::is_gem_code(code) {
        Some("Gem")
    } else if is_rune_code(code) {
        Some("Rune")
    } else if matches!(code, "rin" | "amu") {
        Some("Jewelry")
    } else {
        None
    };
    if let Some((name, category)) = crate::items::base_item_details(code) {
        return (name.into(), category.to_string());
    }
    for table in [GameTable::Weapons, GameTable::Armor, GameTable::Misc] {
        let Ok(contents) = txt.table(table, 96) else {
            continue;
        };
        let mut rows = contents.lines();
        let Some(header) = rows.next() else { continue };
        let columns: Vec<_> = header.split('\t').collect();
        let Some(code_col) = columns.iter().position(|column| *column == "code") else {
            continue;
        };
        let Some(name_col) = columns.iter().position(|column| *column == "name") else {
            continue;
        };
        for row in rows {
            let values: Vec<_> = row.split('\t').collect();
            if values.get(code_col).copied() == Some(code) {
                let name = values
                    .get(name_col)
                    .copied()
                    .filter(|name| !name.is_empty())
                    .unwrap_or(code);
                let table_category = if table == GameTable::Weapons {
                    "Weapon"
                } else if table == GameTable::Armor {
                    "Armor"
                } else {
                    "Item"
                };
                return (
                    name.to_string(),
                    category.unwrap_or(table_category).to_string(),
                );
            }
        }
    }
    (code.to_string(), "Item".into())
}

fn is_rune_code(code: &str) -> bool {
    code.strip_prefix('r')
        .and_then(|suffix| suffix.parse::<u8>().ok())
        .is_some_and(|number| (1..=33).contains(&number))
}

fn display_stat_name(stat: &str) -> String {
    match stat {
        "item_fastercastrate" => "Item Faster Cast Rate".into(),
        "item_fastermovevelocity" => "Item Faster Run/Walk".into(),
        "item_fasterattackrate" => "Item Increased Attack Speed".into(),
        "item_fastergethitrate" => "Item Faster Hit Recovery".into(),
        "item_fasterblockrate" => "Item Faster Block Rate".into(),
        "item_manaafterkill" => "Item Mana After Each Kill".into(),
        "item_healafterkill" => "Item Life After Each Kill".into(),
        "item_healafterdemonkill" => "Item Life After Each Demon Kill".into(),
        "item_goldbonus" => "Item Extra Gold from Monsters".into(),
        "item_magicbonus" => "Item Better Chance of Getting Magic Items".into(),
        "item_nonclassskill" => "Item Non-Class Skill".into(),
        "item_restinpeace" => "Item Slain Monsters Rest in Peace".into(),
        "item_poisonlengthresist" => "Item Poison Length Reduced".into(),
        "item_ignoretargetac" => "Item Ignore Target Defense".into(),
        "item_fractionaltargetac" => "Item Target Defense".into(),
        "item_preventheal" => "Item Prevent Monster Heal".into(),
        "item_halffreezeduration" => "Item Half Freeze Duration".into(),
        "item_elemskill" => "Item Elemental Skill Damage".into(),
        "item_indesctructible" => "Item Indestructible".into(),
        "item_cannotbefrozen" => "Item Cannot Be Frozen".into(),
        "item_staminadrainpct" => "Item Slower Stamina Drain".into(),
        "item_absorbfire" => "Item Fire Absorb".into(),
        "item_absorbfire_percent" => "Item Fire Absorb Percent".into(),
        "item_absorblight" => "Item Lightning Absorb".into(),
        "item_absorblight_percent" => "Item Lightning Absorb Percent".into(),
        "item_absorbcold" => "Item Cold Absorb".into(),
        "item_absorbcold_percent" => "Item Cold Absorb Percent".into(),
        "item_absorbmagic" => "Item Magic Absorb".into(),
        "item_absorbmagic_percent" => "Item Magic Absorb Percent".into(),
        "item_kickdamage" => "Item Kick Damage".into(),
        "item_numsockets" => "Item Number of Sockets".into(),
        "item_stupidity" => "Item Hit Causes Monster to Flee".into(),
        "item_attackertakeslightdamage" => "Item Attacker Takes Lightning Damage".into(),
        "item_magicarrow" => "Item Fires Magic Arrows".into(),
        "item_explosivearrow" => "Item Fires Explosive Arrows".into(),
        "item_extrablood" => "Item Monsters Open Wounds".into(),
        "item_reducedprices" => "Item Reduces Vendor Prices".into(),
        "item_doubleherbduration" => "Item Double Herb Duration".into(),
        "item_addclassskills" => "Item Class Skills".into(),
        "item_skillondeath" => "Item Skill on Death".into(),
        "damageaura" => "Aura Damage".into(),
        "firemindam" => "Fire Minimum Damage".into(),
        "firemaxdam" => "Fire Maximum Damage".into(),
        "lightmindam" => "Lightning Minimum Damage".into(),
        "lightmaxdam" => "Lightning Maximum Damage".into(),
        "magicmindam" => "Magic Minimum Damage".into(),
        "magicmaxdam" => "Magic Maximum Damage".into(),
        "coldmindam" => "Cold Minimum Damage".into(),
        "coldmaxdam" => "Cold Maximum Damage".into(),
        "poisonmindam" => "Poison Minimum Damage".into(),
        "poisonmaxdam" => "Poison Maximum Damage".into(),
        "lifedrainmindam" => "Life Drain Minimum Damage".into(),
        "lifedrainmaxdam" => "Life Drain Maximum Damage".into(),
        "manadrainmindam" => "Mana Drain Minimum Damage".into(),
        "manadrainmaxdam" => "Mana Drain Maximum Damage".into(),
        "stamdrainmindam" => "Stamina Drain Minimum Damage".into(),
        "stamdrainmaxdam" => "Stamina Drain Maximum Damage".into(),
        "stunlength" => "Stun Length".into(),
        "fireresist" => "Fire resistance".into(),
        "coldresist" => "Cold resistance".into(),
        "lightresist" => "Lightning resistance".into(),
        "poisonresist" => "Poison resistance".into(),
        "maxhp" => "Life".into(),
        "maxmana" => "Mana".into(),
        "allskills" => "All skills".into(),
        other => other
            .split('_')
            .map(display_stat_word)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn display_stat_word(part: &str) -> String {
    // ItemStatCost identifiers often concatenate multiple words (for example,
    // `fastercastrate`). Split known Diablo stat terms so their UI labels stay readable.
    const WORDS: &[&str] = &[
        "velocity",
        "recovery",
        "duration",
        "monster",
        "monsters",
        "vendor",
        "prices",
        "chance",
        "getting",
        "magic",
        "items",
        "slain",
        "reduced",
        "frozen",
        "freeze",
        "stamina",
        "drain",
        "double",
        "herb",
        "class",
        "nonclass",
        "skill",
        "skills",
        "attack",
        "faster",
        "block",
        "rate",
        "after",
        "each",
        "heal",
        "kill",
        "demon",
        "gold",
        "bonus",
        "rest",
        "peace",
        "poison",
        "length",
        "resist",
        "ignore",
        "target",
        "defense",
        "prevent",
        "half",
        "elemental",
        "elem",
        "indesctructible",
        "arrow",
        "arrows",
        "explosive",
        "extra",
        "blood",
        "open",
        "wounds",
        "add",
        "find",
        "for",
        "of",
        "from",
        "slower",
        "damage",
        "strength",
        "dexterity",
        "resistance",
        "percent",
        "maximum",
        "minimum",
        "increase",
        "enhanced",
        "against",
        "undead",
        "lightning",
        "physical",
        "absorb",
        "replenish",
        "regenerate",
        "experience",
        "requirements",
        "attacker",
        "takes",
        "thorns",
        "pierce",
        "level",
        "per",
        "hit",
        "cast",
        "move",
        "run",
        "walk",
        "cold",
        "fire",
        "life",
        "mana",
        "item",
        "single",
        "all",
        "speed",
        "to",
        "on",
        "howl",
        "knockback",
        "crushing",
        "blow",
        "deadly",
        "strike",
        "leech",
        "steal",
        "light",
        "radius",
        "thunder",
        "frost",
        "nova",
        "charges",
        "charge",
        "indestructible",
        "ethereal",
        "socket",
        "sockets",
        "enhance",
        "armor",
        "max",
        "min",
        "num",
        "color",
        "normal",
        "time",
        "req",
        "pct",
        "in",
        "cannot",
        "be",
        "ac",
        "up",
        "struck",
        "causes",
        "flee",
        "stupidity",
        "throwable",
        "reanimate",
        "regen",
        "by",
        "thrust",
        "charged",
        "quantity",
        "durability",
        "energy",
        "vitality",
        "accuracy",
        "gems",
        "attackrating",
        "bone",
        "points",
        "new",
        "iron",
        "maiden",
        "last",
        "frame",
        "player",
        "count",
        "weapon",
        "chilling",
        "concentration",
        "conviction",
        "decrepify",
        "hand",
        "athena",
        "do",
        "overlay",
        "framerate",
        "burning",
        "progressive",
        "conversion",
        "modifierlist",
        "secondary",
        "reduction",
        "unit",
        "alignment",
        "get",
    ];

    let mut remaining = part;
    let mut words = Vec::new();
    while !remaining.is_empty() {
        let Some(word) = WORDS
            .iter()
            .filter(|word| remaining.starts_with(**word))
            .max_by_key(|word| word.len())
        else {
            return title_stat_part(part);
        };
        words.push(*word);
        remaining = &remaining[word.len()..];
    }

    words
        .into_iter()
        .map(|word| match word {
            "to" => "to".to_string(),
            "per" => "per".to_string(),
            "on" => "on".to_string(),
            "all" => "All".to_string(),
            "undead" => "Undead".to_string(),
            "ethereal" => "Ethereal".to_string(),
            "indestructible" => "Indestructible".to_string(),
            other => title_stat_part(other),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_stat_part(part: &str) -> String {
    let mut chars = part.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}

#[command]
pub fn bank_export_items(
    app: tauri::AppHandle,
    item_ids: Vec<i64>,
    character_path: String,
    destination: String,
) -> Result<String, String> {
    ensure_diablo_closed()?;
    if item_ids.is_empty() {
        return Err("select at least one bank item".into());
    }
    let store_page = match destination.as_str() {
        "inventory" => StorePage::Inventory,
        "stash" => StorePage::Stash,
        _ => return Err("destination must be inventory or stash".into()),
    };
    let grid_width = if store_page == StorePage::Inventory {
        10
    } else {
        6
    };
    let grid_height = if store_page == StorePage::Inventory {
        4
    } else {
        8
    };
    let source_path = PathBuf::from(&character_path);
    let output_path = source_path.clone();
    let base_bytes = fs::read(&source_path).map_err(|e| {
        format!(
            "could not read character save {}: {e}",
            source_path.display()
        )
    })?;
    let mut save = D2Save::from_bytes(&base_bytes)
        .map_err(|e| format!("could not decode character save: {e}"))?;
    let txt = TxtData::embedded();
    let mut occupied = vec![false; grid_width * grid_height];
    for item in &save.items.0 {
        if item.position.mode == ItemMode::Stored && item.position.store_page == store_page {
            let info = txt
                .item_info(item.code, save.version)
                .map_err(|e| format!("could not size existing {}: {e}", item.code_string()))?;
            if let (Some(width), Some(height)) = (info.inv_width, info.inv_height) {
                mark_grid(
                    &mut occupied,
                    grid_width,
                    grid_height,
                    item.position.inv_x as usize,
                    item.position.inv_y as usize,
                    width as usize,
                    height as usize,
                );
            }
        }
    }

    let conn = open_app_db(&app)?;
    let mut added_seeds = std::collections::HashSet::new();
    let mut transferred = Vec::with_capacity(item_ids.len());
    for id in &item_ids {
        let (item_data, transferred_to): (Option<Vec<u8>>, Option<String>) = conn
            .query_row(
                "SELECT item_data, transferred_to FROM bank_items WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| format!("bank item {id} is unavailable: {e}"))?;
        if let Some(path) = transferred_to {
            return Err(format!("bank item {id} was already transferred to {path}"));
        }
        let item_data =
            item_data.ok_or_else(|| format!("bank item {id} has no encoded game item data"))?;
        let mut reader = BitReader::new(&item_data);
        let mut item = Item::read(&mut reader, txt, save.version)
            .map_err(|e| format!("bank item {id} could not be decoded: {e}"))?;
        if item.seed != 0
            && (save
                .items
                .0
                .iter()
                .any(|existing| existing.seed == item.seed)
                || !added_seeds.insert(item.seed))
        {
            return Err(format!(
                "{} is already present in this save by item fingerprint",
                item.code_string()
            ));
        }
        let info = txt
            .item_info(item.code, save.version)
            .map_err(|e| format!("could not size {}: {e}", item.code_string()))?;
        let width = info.inv_width.unwrap_or(1) as usize;
        let height = info.inv_height.unwrap_or(1) as usize;
        let (x, y) = find_grid_space(&mut occupied, grid_width, grid_height, width, height)
            .ok_or_else(|| {
                format!(
                    "not enough room in the character {destination} for {}",
                    item.code_string()
                )
            })?;
        item.position = ItemPosition {
            mode: ItemMode::Stored,
            body_location: BodyLocation::None,
            inv_x: x as u8,
            inv_y: y as u8,
            store_page,
            ..Default::default()
        };
        save.items.0.push(item);
        transferred.push(*id);
    }

    let output_bytes = save
        .to_bytes()
        .map_err(|e| format!("could not encode updated character save: {e}"))?;
    let check = D2Save::from_bytes(&output_bytes)
        .map_err(|e| format!("updated save failed validation: {e}"))?;
    if check.items.len() != save.items.len() {
        return Err("updated save failed item-count verification".into());
    }

    capture_character_backup(&app, &output_path, true)?;
    let now = now_secs()?;
    let temp_path = unique_sidecar_path(&output_path, "tmp", now);
    fs::write(&temp_path, &output_bytes)
        .map_err(|e| format!("could not write updated save: {e}"))?;
    if fs::read(&temp_path).ok() != Some(output_bytes.clone()) {
        let _ = fs::remove_file(&temp_path);
        return Err("updated save byte verification failed".into());
    }
    ensure_diablo_closed()?;
    fs::rename(&temp_path, &output_path)
        .map_err(|e| format!("could not finalize updated save: {e}"))?;
    update_character_save_state(&app, &output_path, &output_bytes)?;

    let now = now_secs()?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| format!("save updated but could not record item transfer: {e}"))?;
    for id in transferred {
        tx.execute(
            "UPDATE bank_items SET transferred_to = ?1, updated_at = ?2 WHERE id = ?3 AND transferred_to IS NULL",
            rusqlite::params![output_path.to_string_lossy(), now, id],
        ).map_err(|e| format!("save created but bank item {id} status could not be updated: {e}"))?;
    }
    tx.commit()
        .map_err(|e| format!("save created but transfer status could not be committed: {e}"))?;
    Ok(output_path.to_string_lossy().to_string())
}
