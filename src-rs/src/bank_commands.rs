//! Bank persistence and verified transfers between stash, bank, and characters.

use crate::bank::BankItem;
use crate::d2x::{empty_d2x_stash, read_transferable_stash_items};
use crate::database::open_app_db;
use crate::grid::{find_grid_space, mark_grid};
use crate::{capture_character_backup, now_secs, unique_sidecar_path, update_character_save_state};
use ::d2s::{
    BodyLocation, D2Save, Item, ItemMode, ItemPosition, StorePage, TxtData,
    bits::BitReader,
    data::{ExternalData, GameTable},
};
use serde::Serialize;
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
        "fireresist" => "Fire resistance".into(),
        "coldresist" => "Cold resistance".into(),
        "lightresist" => "Lightning resistance".into(),
        "poisonresist" => "Poison resistance".into(),
        "maxhp" => "Life".into(),
        "maxmana" => "Mana".into(),
        "allskills" => "All skills".into(),
        other => other
            .split('_')
            .map(|part| {
                let mut chars = part.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                })
            })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

#[command]
pub fn bank_export_items(
    app: tauri::AppHandle,
    item_ids: Vec<i64>,
    character_path: String,
    destination: String,
) -> Result<String, String> {
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
