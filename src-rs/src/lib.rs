mod bank;
mod bank_commands;
mod d2s;
mod d2x;
mod database;
mod grid;
mod items;

use ::d2s::{D2Save, StatId};
use bank_commands::{
    bank_add_item, bank_delete_item, bank_export_items, bank_get_items, bank_import_stash_items,
};
use d2s::{CharacterData, parse_d2s};
use d2x::{StashData, parse_d2x};
use database::{get_app_setting, open_app_db, set_app_setting};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Manager, command};

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "data")]
enum ParseResult<T> {
    Ok(T),
    Err(String),
}

#[derive(Debug, Serialize)]
struct CharacterBuildData {
    class_code: String,
    skill_levels: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct SkillCatalog {
    #[serde(rename = "skillOffset")]
    skill_offset: u16,
    skills: Vec<SkillDefinition>,
}

#[derive(Debug, Deserialize)]
struct SkillDefinition {
    id: u16,
    name: String,
    #[serde(rename = "requiredLevel")]
    required_level: u8,
    #[serde(rename = "maxLevel")]
    max_level: u8,
    prerequisites: Vec<usize>,
}

const CLASSIC_SKILL_CATALOG: &str = include_str!("../../src-ts/data/classic-skills-v96.json");

fn class_code(class_id: u8) -> Option<&'static str> {
    ["ama", "sor", "nec", "pal", "bar", "dru", "ass"]
        .get(class_id as usize)
        .copied()
}

fn base_attributes(class_id: u8) -> Option<[u16; 4]> {
    // Stored order: strength, dexterity, vitality, energy.
    [
        [20, 25, 20, 15], // Amazon
        [10, 25, 10, 35], // Sorceress
        [15, 25, 15, 25], // Necromancer
        [25, 20, 25, 15], // Paladin
        [30, 20, 25, 10], // Barbarian
        [15, 20, 25, 20], // Druid
        [20, 20, 20, 25], // Assassin
    ]
    .get(class_id as usize)
    .copied()
}

/// Lightweight character listing for the library view
#[derive(Debug, Clone, Serialize)]
pub struct CharacterListing {
    pub path: String,
    pub name: String,
    pub class: String,
    pub level: u8,
    pub file_size: u64,
    pub modified: u64,
    pub backup_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CharacterBackupVersion {
    pub id: i64,
    pub created_at: i64,
    pub file_size: u64,
}

fn now_secs() -> Result<i64, String> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("time error: {}", e))?
        .as_secs() as i64)
}

fn unique_sidecar_path(path: &Path, label: &str, timestamp: i64) -> PathBuf {
    let base = format!("{}.{}-{timestamp}", path.display(), label);
    let mut candidate = PathBuf::from(&base);
    let mut suffix = 1u32;
    while candidate.exists() {
        candidate = PathBuf::from(format!("{base}-{suffix}"));
        suffix += 1;
    }
    candidate
}

#[command]
fn parse_character(path: String) -> ParseResult<CharacterData> {
    match parse_d2s(&path) {
        Ok(data) => ParseResult::Ok(data),
        Err(e) => ParseResult::Err(e.to_string()),
    }
}

#[command]
fn parse_stash(path: String) -> ParseResult<StashData> {
    match parse_d2x(&path) {
        Ok(data) => ParseResult::Ok(data),
        Err(e) => ParseResult::Err(e.to_string()),
    }
}

#[command]
fn scan_characters(
    app: tauri::AppHandle,
    dir: String,
    auto_backups: bool,
) -> ParseResult<Vec<CharacterListing>> {
    let path = Path::new(&dir);
    if !path.is_dir() {
        return ParseResult::Err(format!("not a directory: {}", dir));
    }

    let mut listings: Vec<CharacterListing> = Vec::new();

    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.extension().and_then(|e| e.to_str()) != Some("d2s") {
                    continue;
                }

                let file_name = file_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let file_size = metadata.len();
                let modified = metadata
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    })
                    .unwrap_or(0);

                let parsed = parse_d2s(file_path.to_str().unwrap_or(""));
                let backup_warning = if auto_backups {
                    match &parsed {
                        Ok(_) => observe_character_save(&app, &file_path).err(),
                        Err(error) => Some(format!("save could not be parsed for backup: {error}")),
                    }
                } else {
                    None
                };
                let (name, class, level) = match &parsed {
                    Ok(data) => (data.name.clone(), format!("{:?}", data.class), data.level),
                    Err(_) => (file_name.replace(".d2s", ""), "Unknown".to_string(), 0),
                };

                listings.push(CharacterListing {
                    path: file_path.to_string_lossy().to_string(),
                    name,
                    class,
                    level,
                    file_size,
                    modified,
                    backup_warning,
                });
            }
            listings.sort_by_key(|listing| std::cmp::Reverse(listing.modified));
        }
        Err(e) => {
            return ParseResult::Err(format!("failed to read directory: {}", e));
        }
    }

    ParseResult::Ok(listings)
}

fn observe_character_save(app: &tauri::AppHandle, character_path: &Path) -> Result<(), String> {
    let bytes =
        fs::read(character_path).map_err(|e| format!("could not read character save: {e}"))?;
    D2Save::from_bytes(&bytes)
        .map_err(|e| format!("character save could not be validated for backup: {e}"))?;
    let fingerprint = content_fingerprint(&bytes);
    let path = character_path.to_string_lossy().to_string();
    let conn = open_app_db(app)?;
    let previous: Option<String> = conn
        .query_row(
            "SELECT content_fingerprint FROM character_save_state WHERE original_path = ?1",
            [&path],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("could not read character backup state: {e}"))?;
    match previous {
        None => {
            conn.execute(
                "INSERT INTO character_save_state (original_path, content_fingerprint) VALUES (?1, ?2)",
                rusqlite::params![path, fingerprint],
            )
            .map_err(|e| format!("could not initialize character backup state: {e}"))?;
        }
        Some(previous) if previous != fingerprint => {
            capture_character_backup(app, character_path, false)?;
            conn.execute(
                "UPDATE character_save_state SET content_fingerprint = ?1 WHERE original_path = ?2",
                rusqlite::params![fingerprint, path],
            )
            .map_err(|e| format!("could not update character backup state: {e}"))?;
        }
        Some(_) => {}
    }
    Ok(())
}

fn update_character_save_state(
    app: &tauri::AppHandle,
    character_path: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    let conn = open_app_db(app)?;
    conn.execute(
        "INSERT INTO character_save_state (original_path, content_fingerprint) VALUES (?1, ?2)
         ON CONFLICT(original_path) DO UPDATE SET content_fingerprint = excluded.content_fingerprint",
        rusqlite::params![character_path.to_string_lossy(), content_fingerprint(bytes)],
    )
    .map_err(|e| format!("could not update character backup state: {e}"))?;
    Ok(())
}

fn content_fingerprint(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn character_backup_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {e}"))?;
    let root = app_dir.join("character-backups");
    fs::create_dir_all(&root).map_err(|e| format!("failed to create backup directory: {e}"))?;
    Ok(root)
}

fn capture_character_backup(
    app: &tauri::AppHandle,
    character_path: &Path,
    force: bool,
) -> Result<Option<CharacterBackupVersion>, String> {
    let original_path = character_path.to_string_lossy().to_string();
    let bytes = fs::read(character_path).map_err(|e| {
        format!(
            "could not read character save {}: {e}",
            character_path.display()
        )
    })?;
    D2Save::from_bytes(&bytes)
        .map_err(|e| format!("character save could not be validated before backup: {e}"))?;
    let fingerprint = content_fingerprint(&bytes);
    let conn = open_app_db(app)?;
    let latest_fingerprint: Option<String> = conn
        .query_row(
            "SELECT content_fingerprint FROM character_backups WHERE original_path = ?1 ORDER BY created_at DESC, id DESC LIMIT 1",
            [&original_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("could not inspect character backup history: {e}"))?;
    if !force && latest_fingerprint.as_deref() == Some(fingerprint.as_str()) {
        return Ok(None);
    }

    let path_key = content_fingerprint(original_path.as_bytes());
    let folder = character_backup_root(app)?.join(path_key);
    fs::create_dir_all(&folder).map_err(|e| format!("could not create character history: {e}"))?;
    let timestamp = now_secs()?;
    let base = folder.join(format!("save-{timestamp}-{fingerprint}.d2s"));
    let mut snapshot_path = base.clone();
    let mut suffix = 1;
    while snapshot_path.exists() {
        snapshot_path = folder.join(format!("save-{timestamp}-{fingerprint}-{suffix}.d2s"));
        suffix += 1;
    }
    fs::write(&snapshot_path, &bytes)
        .map_err(|e| format!("could not write character backup: {e}"))?;
    let verified_bytes =
        fs::read(&snapshot_path).map_err(|e| format!("could not verify character backup: {e}"))?;
    if verified_bytes != bytes || content_fingerprint(&verified_bytes) != fingerprint {
        let _ = fs::remove_file(&snapshot_path);
        return Err("character backup verification failed".into());
    }
    D2Save::from_bytes(&verified_bytes)
        .map_err(|e| format!("character backup failed save validation: {e}"))?;
    let created_at = now_secs()?;
    let file_size = i64::try_from(bytes.len())
        .map_err(|_| "character save is too large to record in backup history".to_string())?;
    conn.execute(
        "INSERT INTO character_backups (original_path, snapshot_path, content_fingerprint, file_size, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![original_path, snapshot_path.to_string_lossy(), fingerprint, file_size, created_at],
    )
    .map_err(|e| format!("could not record character backup: {e}"))?;
    Ok(Some(CharacterBackupVersion {
        id: conn.last_insert_rowid(),
        created_at,
        file_size: file_size as u64,
    }))
}

#[command]
fn character_backup_history(
    app: tauri::AppHandle,
    character_path: String,
) -> Result<Vec<CharacterBackupVersion>, String> {
    let conn = open_app_db(&app)?;
    let mut statement = conn
        .prepare("SELECT id, created_at, file_size FROM character_backups WHERE original_path = ?1 ORDER BY created_at DESC, id DESC")
        .map_err(|e| format!("could not read character backup history: {e}"))?;
    statement
        .query_map([character_path], |row| {
            let file_size = row.get::<_, i64>(2)?;
            let file_size = u64::try_from(file_size).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })?;
            Ok(CharacterBackupVersion {
                id: row.get(0)?,
                created_at: row.get::<_, i64>(1)?,
                file_size,
            })
        })
        .map_err(|e| format!("could not read character backup history: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("could not read character backup history: {e}"))
}

#[command]
fn character_restore_backup(
    app: tauri::AppHandle,
    character_path: String,
    backup_id: i64,
) -> Result<String, String> {
    let conn = open_app_db(&app)?;
    let (snapshot_path, expected_fingerprint): (String, String) = conn
        .query_row(
            "SELECT snapshot_path, content_fingerprint FROM character_backups WHERE id = ?1 AND original_path = ?2",
            rusqlite::params![backup_id, character_path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("backup version was not found: {e}"))?;
    let snapshot_bytes =
        fs::read(&snapshot_path).map_err(|e| format!("could not read backup version: {e}"))?;
    if content_fingerprint(&snapshot_bytes) != expected_fingerprint {
        return Err("selected backup failed its integrity check".into());
    }
    D2Save::from_bytes(&snapshot_bytes)
        .map_err(|e| format!("selected backup is not a valid character save: {e}"))?;

    let target_path = PathBuf::from(&character_path);
    if !target_path.is_file() {
        return Err("character save no longer exists at this path".into());
    }
    let current_bytes = fs::read(&target_path)
        .map_err(|e| format!("could not reread current character save: {e}"))?;
    if content_fingerprint(&current_bytes) == expected_fingerprint {
        return Ok("selected backup already matches the character save".into());
    }
    capture_character_backup(&app, &target_path, true)?;
    let timestamp = now_secs()?;
    let temp_path = unique_sidecar_path(&target_path, "restore-tmp", timestamp);
    let displaced_path = unique_sidecar_path(&target_path, "restore-current", timestamp);
    fs::write(&temp_path, &snapshot_bytes)
        .map_err(|e| format!("could not prepare restored save: {e}"))?;
    fs::rename(&target_path, &displaced_path)
        .map_err(|e| format!("could not preserve current character save: {e}"))?;
    if let Err(error) = fs::rename(&temp_path, &target_path) {
        let _ = fs::rename(&displaced_path, &target_path);
        let _ = fs::remove_file(&temp_path);
        return Err(format!("could not restore character save: {error}"));
    }
    let restored = match fs::read(&target_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = fs::remove_file(&target_path);
            let _ = fs::rename(&displaced_path, &target_path);
            return Err(format!(
                "restored save could not be reread; previous save was put back: {error}"
            ));
        }
    };
    let valid = restored == snapshot_bytes
        && content_fingerprint(&restored) == expected_fingerprint
        && D2Save::from_bytes(&restored).is_ok();
    if !valid {
        let _ = fs::remove_file(&target_path);
        let _ = fs::rename(&displaced_path, &target_path);
        return Err("restored save failed verification; the previous save was put back".into());
    }
    fs::remove_file(&displaced_path)
        .map_err(|e| format!("restored save is valid, but temporary cleanup failed: {e}"))?;
    conn.execute(
        "INSERT INTO character_save_state (original_path, content_fingerprint) VALUES (?1, ?2)
         ON CONFLICT(original_path) DO UPDATE SET content_fingerprint = excluded.content_fingerprint",
        rusqlite::params![character_path, expected_fingerprint],
    )
    .map_err(|e| format!("restore succeeded but backup state could not be updated: {e}"))?;
    Ok(format!(
        "restored character save at {}",
        target_path.display()
    ))
}

#[command]
fn character_build_data(character_path: String) -> Result<CharacterBuildData, String> {
    let bytes =
        fs::read(&character_path).map_err(|e| format!("could not read character save: {e}"))?;
    let save =
        D2Save::from_bytes(&bytes).map_err(|e| format!("could not decode character save: {e}"))?;
    if save.version != 96 {
        return Err(
            "skill and attribute editing currently supports classic LoD version 96 saves".into(),
        );
    }
    let class_id = save.character.class.to_raw();
    let code = class_code(class_id).ok_or("this character class is not supported")?;
    if save.skills.class.to_raw() != class_id || save.skills.levels.len() != 30 {
        return Err(
            "character skill section does not match its class or has an unsupported length".into(),
        );
    }
    Ok(CharacterBuildData {
        class_code: code.into(),
        skill_levels: save.skills.levels,
    })
}

#[command]
fn edit_character_build(
    app: tauri::AppHandle,
    character_path: String,
    attributes: [u16; 4],
    skill_levels: Vec<u8>,
) -> Result<String, String> {
    const ATTRIBUTE_MAX: u16 = 1023;
    let path = PathBuf::from(&character_path);
    if !path.is_file() {
        return Err("selected character save does not exist".into());
    }
    let original_bytes =
        fs::read(&path).map_err(|e| format!("could not read character save: {e}"))?;
    let mut save = D2Save::from_bytes(&original_bytes)
        .map_err(|e| format!("could not decode character save: {e}"))?;
    if save.version != 96 {
        return Err(
            "skill and attribute editing currently supports classic LoD version 96 saves".into(),
        );
    }
    if skill_levels.len() != 30 || save.skills.levels.len() != 30 {
        return Err("exactly 30 skill levels are required for this character class".into());
    }
    if attributes.iter().any(|value| *value > ATTRIBUTE_MAX) {
        return Err(format!("attributes cannot exceed {ATTRIBUTE_MAX}"));
    }

    let class_id = save.character.class.to_raw();
    let code = class_code(class_id).ok_or("this character class is not supported")?;
    if save.skills.class.to_raw() != class_id {
        return Err("character skill section does not match the character class".into());
    }
    let bases =
        base_attributes(class_id).ok_or("base attributes are unavailable for this class")?;
    let catalog: std::collections::HashMap<String, SkillCatalog> =
        serde_json::from_str(CLASSIC_SKILL_CATALOG)
            .map_err(|e| format!("built-in skill data is invalid: {e}"))?;
    let class_skills = catalog
        .get(code)
        .ok_or("built-in skill data is missing this class")?;
    if class_skills.skills.len() != 30 {
        return Err("built-in skill data has an invalid number of class skills".into());
    }

    let stat_ids = [
        StatId::STRENGTH,
        StatId::DEXTERITY,
        StatId::VITALITY,
        StatId::ENERGY,
    ];
    let current_attributes: [u16; 4] =
        stat_ids.map(|id| save.stats.get(id).clamp(0, u16::MAX as i64) as u16);
    for index in 0..4 {
        if current_attributes[index] < bases[index] || attributes[index] < bases[index] {
            return Err("attributes cannot be reduced below this class's starting values".into());
        }
    }
    let old_stat_points = save
        .stats
        .get(StatId::STAT_POINTS)
        .clamp(0, u16::MAX as i64) as u16;
    let earned_stat_budget = current_attributes
        .iter()
        .zip(bases)
        .map(|(value, base)| value - base)
        .sum::<u16>()
        .checked_add(old_stat_points)
        .ok_or("stat point total overflowed")?;
    let new_spent_stats = attributes
        .iter()
        .zip(bases)
        .map(|(value, base)| value - base)
        .sum::<u16>();
    let new_stat_points = earned_stat_budget
        .checked_sub(new_spent_stats)
        .ok_or("these attributes use more points than the character has earned")?;

    let old_skill_points = save
        .stats
        .get(StatId::SKILL_POINTS)
        .clamp(0, u16::MAX as i64) as u16;
    let old_spent_skills = save
        .skills
        .levels
        .iter()
        .map(|level| *level as u16)
        .sum::<u16>();
    let earned_skill_budget = old_spent_skills
        .checked_add(old_skill_points)
        .ok_or("skill point total overflowed")?;
    let new_spent_skills = skill_levels.iter().map(|level| *level as u16).sum::<u16>();
    let new_skill_points = earned_skill_budget
        .checked_sub(new_spent_skills)
        .ok_or("these skills use more points than the character has earned")?;

    for (index, (level, definition)) in skill_levels.iter().zip(&class_skills.skills).enumerate() {
        if definition.id != class_skills.skill_offset + index as u16 {
            return Err("built-in skill ids do not match the save skill layout".into());
        }
        if *level > definition.max_level {
            return Err(format!(
                "{} cannot exceed {} hard points",
                definition.name, definition.max_level
            ));
        }
        if *level > 0 {
            if save.character.level < definition.required_level {
                return Err(format!(
                    "{} requires character level {}",
                    definition.name, definition.required_level
                ));
            }
            for prerequisite in &definition.prerequisites {
                if skill_levels.get(*prerequisite).copied().unwrap_or(0) == 0 {
                    return Err(format!(
                        "{} requires at least one point in its prerequisite skills",
                        definition.name
                    ));
                }
            }
        }
    }

    if current_attributes == attributes && save.skills.levels == skill_levels {
        return Ok(path.to_string_lossy().into_owned());
    }
    for (id, value) in stat_ids.into_iter().zip(attributes) {
        save.stats.set(id, value as i64);
    }
    save.stats.set(StatId::STAT_POINTS, new_stat_points as i64);
    save.skills.levels.clone_from(&skill_levels);
    save.stats
        .set(StatId::SKILL_POINTS, new_skill_points as i64);

    let updated_bytes = save
        .to_bytes()
        .map_err(|e| format!("could not encode updated character: {e}"))?;
    let verified = D2Save::from_bytes(&updated_bytes)
        .map_err(|e| format!("updated save failed validation: {e}"))?;
    if verified.skills.levels != skill_levels
        || verified.stats.get(StatId::STAT_POINTS) != new_stat_points as i64
        || verified.stats.get(StatId::SKILL_POINTS) != new_skill_points as i64
        || stat_ids
            .iter()
            .zip(attributes)
            .any(|(id, value)| verified.stats.get(*id) != value as i64)
    {
        return Err("updated save failed attribute and skill verification".into());
    }

    capture_character_backup(&app, &path, true)?;
    let timestamp = now_secs()?;
    let temp_path = unique_sidecar_path(&path, "build-tmp", timestamp);
    let displaced_path = unique_sidecar_path(&path, "build-previous", timestamp);
    fs::write(&temp_path, &updated_bytes)
        .map_err(|e| format!("could not prepare updated save: {e}"))?;
    if fs::read(&temp_path).ok() != Some(updated_bytes.clone()) {
        let _ = fs::remove_file(&temp_path);
        return Err("updated character save failed byte verification".into());
    }
    fs::rename(&path, &displaced_path)
        .map_err(|e| format!("could not preserve current character save: {e}"))?;
    if let Err(error) = fs::rename(&temp_path, &path) {
        let _ = fs::rename(&displaced_path, &path);
        let _ = fs::remove_file(&temp_path);
        return Err(format!("could not write updated character save: {error}"));
    }
    let written =
        fs::read(&path).map_err(|e| format!("updated character save could not be reread: {e}"))?;
    let final_save =
        D2Save::from_bytes(&written).map_err(|e| format!("written save failed validation: {e}"))?;
    let final_valid = written == updated_bytes
        && final_save.skills.levels == skill_levels
        && final_save.stats.get(StatId::STAT_POINTS) == new_stat_points as i64
        && final_save.stats.get(StatId::SKILL_POINTS) == new_skill_points as i64
        && stat_ids
            .iter()
            .zip(attributes)
            .all(|(id, value)| final_save.stats.get(*id) == value as i64);
    if !final_valid {
        let _ = fs::remove_file(&path);
        let _ = fs::rename(&displaced_path, &path);
        return Err(
            "written save failed verification; previous character file was restored".into(),
        );
    }
    fs::remove_file(&displaced_path)
        .map_err(|e| format!("save is valid, but temporary cleanup failed: {e}"))?;
    update_character_save_state(&app, &path, &updated_bytes)?;
    Ok(path.to_string_lossy().into_owned())
}

#[command]
fn edit_character_gold(
    app: tauri::AppHandle,
    character_path: String,
    carried_gold: u32,
    stash_gold: u32,
) -> Result<String, String> {
    const STASH_GOLD_LIMIT: u32 = 2_500_000;
    let selected_path = PathBuf::from(&character_path);
    if !selected_path.is_file() {
        return Err("selected character save does not exist".into());
    }
    let output_path = selected_path.clone();
    let base_bytes = fs::read(&selected_path).map_err(|e| {
        format!(
            "could not read character save {}: {e}",
            selected_path.display()
        )
    })?;
    let mut save = D2Save::from_bytes(&base_bytes)
        .map_err(|e| format!("could not decode character save: {e}"))?;
    if save.version != 96 {
        return Err(
            "gold editing currently supports only classic Lord of Destruction version 96 saves"
                .into(),
        );
    }
    let level = save.character.level as u32;
    if !(1..=99).contains(&level) {
        return Err("character level must be between 1 and 99 before editing gold".into());
    }
    let carried_gold_limit = level * 10_000;
    if carried_gold > carried_gold_limit {
        return Err(format!(
            "carried gold cannot exceed {carried_gold_limit} at level {level}"
        ));
    }
    if stash_gold > STASH_GOLD_LIMIT {
        return Err(format!("stash gold cannot exceed {STASH_GOLD_LIMIT}"));
    }
    let old_carried = save.stats.get(StatId::GOLD).max(0) as u32;
    let old_stash = save.stats.get(StatId::STASH_GOLD).max(0) as u32;
    if old_carried == carried_gold && old_stash == stash_gold {
        return Ok(output_path.to_string_lossy().to_string());
    }

    save.stats.set(StatId::GOLD, carried_gold as i64);
    save.stats.set(StatId::STASH_GOLD, stash_gold as i64);
    let output_bytes = save
        .to_bytes()
        .map_err(|e| format!("could not encode edited character save: {e}"))?;
    let check = D2Save::from_bytes(&output_bytes)
        .map_err(|e| format!("edited save failed validation: {e}"))?;
    if check.stats.get(StatId::GOLD) != carried_gold as i64
        || check.stats.get(StatId::STASH_GOLD) != stash_gold as i64
    {
        return Err("edited save failed gold value verification".into());
    }

    capture_character_backup(&app, &output_path, true)?;
    let timestamp = now_secs()?;
    let temp_path = unique_sidecar_path(&output_path, "gold-tmp", timestamp);
    let displaced_path = unique_sidecar_path(&output_path, "gold-previous", timestamp);
    fs::write(&temp_path, &output_bytes)
        .map_err(|e| format!("could not prepare edited save: {e}"))?;
    if output_path.exists() {
        fs::rename(&output_path, &displaced_path)
            .map_err(|e| format!("could not preserve existing app save: {e}"))?;
    }
    if let Err(error) = fs::rename(&temp_path, &output_path) {
        if displaced_path.exists() {
            let _ = fs::rename(&displaced_path, &output_path);
        }
        let _ = fs::remove_file(&temp_path);
        return Err(format!("could not save edited character file: {error}"));
    }
    let written =
        fs::read(&output_path).map_err(|e| format!("edited save could not be reread: {e}"))?;
    let verified =
        D2Save::from_bytes(&written).map_err(|e| format!("written save failed validation: {e}"))?;
    if written != output_bytes
        || verified.stats.get(StatId::GOLD) != carried_gold as i64
        || verified.stats.get(StatId::STASH_GOLD) != stash_gold as i64
    {
        let _ = fs::remove_file(&output_path);
        if displaced_path.exists() {
            let _ = fs::rename(&displaced_path, &output_path);
        }
        return Err(
            "written save failed verification; previous character file was restored".into(),
        );
    }
    if displaced_path.exists() {
        fs::remove_file(&displaced_path)
            .map_err(|e| format!("save is valid, but temporary cleanup failed: {e}"))?;
    }
    update_character_save_state(&app, &output_path, &output_bytes)?;
    Ok(output_path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_app_setting,
            set_app_setting,
            parse_character,
            parse_stash,
            scan_characters,
            character_backup_history,
            character_restore_backup,
            character_build_data,
            edit_character_build,
            edit_character_gold,
            bank_add_item,
            bank_delete_item,
            bank_get_items,
            bank_import_stash_items,
            bank_export_items,
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
