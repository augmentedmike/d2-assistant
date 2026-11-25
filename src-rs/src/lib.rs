mod bank;
mod d2s;
mod d2x;
mod items;

use ::d2s::{
    BodyLocation, D2Save, Item, ItemMode, ItemPosition, StatId, StorePage, TxtData,
    bits::BitReader,
    data::{ExternalData, GameTable},
};
use bank::BankItem;
use d2s::{CharacterData, parse_d2s};
use d2x::{
    StashData, empty_d2x_stash, parse_d2x, read_transferable_stash_items,
    requirements_from_item_record,
};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Manager, command};

const STASH_SOURCE: &str = "Stash import";

#[derive(Debug, Serialize)]
struct BankImportResult {
    items: Vec<BankItem>,
    backup_path: Option<String>,
    source_cleared: bool,
    source_warning: Option<String>,
}

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
                        Ok(_) => match observe_character_save(&app, &file_path) {
                            Ok(_) => None,
                            Err(error) => Some(error),
                        },
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
            listings.sort_by(|a, b| b.modified.cmp(&a.modified));
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
    conn.execute(
        "INSERT INTO character_backups (original_path, snapshot_path, content_fingerprint, file_size, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![original_path, snapshot_path.to_string_lossy(), fingerprint, bytes.len() as u64, created_at],
    )
    .map_err(|e| format!("could not record character backup: {e}"))?;
    Ok(Some(CharacterBackupVersion {
        id: conn.last_insert_rowid(),
        created_at,
        file_size: bytes.len() as u64,
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
            Ok(CharacterBackupVersion {
                id: row.get(0)?,
                created_at: row.get::<_, i64>(1)?,
                file_size: row.get::<_, u64>(2)?,
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

fn app_db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to get app data dir: {}", e))?;
    fs::create_dir_all(&app_dir).map_err(|e| format!("failed to create app data dir: {}", e))?;
    Ok(app_dir.join("d2a.db"))
}

/// Prefer new db name; migrate from legacy bank.db if needed.
fn open_app_db(app: &tauri::AppHandle) -> Result<rusqlite::Connection, String> {
    let db_path = app_db_path(app)?;
    if !db_path.exists() {
        let legacy = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("failed to get app data dir: {}", e))?
            .join("bank.db");
        if legacy.exists() {
            let _ = fs::rename(&legacy, &db_path);
        }
    }
    let conn = rusqlite::Connection::open(&db_path).map_err(|e| format!("db open error: {}", e))?;
    init_app_db(&conn)?;
    Ok(conn)
}

fn init_app_db(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS character_save_state (
            original_path TEXT PRIMARY KEY,
            content_fingerprint TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("failed to init character save tracking: {e}"))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS character_backups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            original_path TEXT NOT NULL,
            snapshot_path TEXT NOT NULL,
            content_fingerprint TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("failed to init character backup history: {e}"))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS bank_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            base_item TEXT NOT NULL,
            affixes_json TEXT NOT NULL DEFAULT '[]',
            level_req INTEGER NOT NULL DEFAULT 0,
            class_req TEXT NOT NULL DEFAULT 'Any',
            sockets INTEGER NOT NULL DEFAULT 0,
            icon_path TEXT,
            source_character TEXT NOT NULL DEFAULT '',
            banked_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            item_data BLOB,
            transferred_to TEXT,
            source_key TEXT UNIQUE,
            requirements_decoded INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .map_err(|e| format!("failed to init bank table: {}", e))?;

    for (column, definition) in [
        ("item_data", "BLOB"),
        ("transferred_to", "TEXT"),
        ("source_key", "TEXT"),
        ("requirements_decoded", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        let exists: bool = conn
            .prepare("PRAGMA table_info(bank_items)")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| row.get::<_, String>(1))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|e| format!("failed to inspect bank schema: {e}"))?
            .iter()
            .any(|existing| existing == column);
        if !exists {
            conn.execute(
                &format!("ALTER TABLE bank_items ADD COLUMN {column} {definition}"),
                [],
            )
            .map_err(|e| format!("failed to add {column} bank field: {e}"))?;
        }
    }
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS bank_items_source_key ON bank_items(source_key)",
        [],
    )
    .map_err(|e| format!("failed to index bank source records: {e}"))?;
    conn.execute("DROP TABLE IF EXISTS notes", [])
        .map_err(|e| format!("failed to remove app-side project notes: {e}"))?;
    conn.execute(
        "UPDATE bank_items SET source_character = 'Stash import' WHERE source_character = 'ITAMZ (ATMA)'",
        [],
    )
    .map_err(|e| format!("failed to normalize bank import labels: {e}"))?;
    conn.execute(
        "UPDATE bank_items SET source_key = 'D2X:' || substr(source_key, 7) WHERE source_key LIKE 'ITAMZ:%'",
        [],
    )
    .map_err(|e| format!("failed to normalize stash identity keys: {e}"))?;
    conn.execute(
        "UPDATE bank_items SET level_req = -1 WHERE source_key LIKE 'D2X:%' AND level_req = 0 AND class_req = 'Unknown'",
        [],
    )
    .map_err(|e| format!("failed to mark unknown bank requirements: {e}"))?;
    let pending_requirements = conn
        .prepare("SELECT id, item_data FROM bank_items WHERE item_data IS NOT NULL AND requirements_decoded = 0")
        .and_then(|mut statement| {
            statement
                .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|e| format!("failed to load bank items for requirement decoding: {e}"))?;
    for (id, item_data) in pending_requirements {
        let (level_req, class_req) =
            requirements_from_item_record(&item_data).unwrap_or_else(|_| (-1, "Unknown".into()));
        conn.execute(
            "UPDATE bank_items SET level_req = ?1, class_req = ?2, requirements_decoded = 1 WHERE id = ?3",
            rusqlite::params![level_req, class_req, id],
        )
        .map_err(|e| format!("failed to save decoded bank requirements: {e}"))?;
    }

    Ok(())
}

#[command]
fn bank_add_item(
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
fn bank_get_items(app: tauri::AppHandle) -> Result<Vec<BankItem>, String> {
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
fn bank_delete_item(app: tauri::AppHandle, id: i64) -> Result<(), String> {
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
fn bank_import_stash_items(
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
fn bank_export_items(
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

fn mark_grid(
    grid: &mut [bool],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    item_width: usize,
    item_height: usize,
) {
    for row in y..(y + item_height).min(height) {
        for column in x..(x + item_width).min(width) {
            grid[row * width + column] = true;
        }
    }
}

fn find_grid_space(
    grid: &mut [bool],
    width: usize,
    height: usize,
    item_width: usize,
    item_height: usize,
) -> Option<(usize, usize)> {
    if item_width > width || item_height > height {
        return None;
    }
    for y in 0..=height - item_height {
        for x in 0..=width - item_width {
            let empty = (y..y + item_height)
                .all(|row| (x..x + item_width).all(|column| !grid[row * width + column]));
            if empty {
                mark_grid(grid, width, height, x, y, item_width, item_height);
                return Some((x, y));
            }
        }
    }
    None
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
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
