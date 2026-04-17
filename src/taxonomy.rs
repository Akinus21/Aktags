use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::{taxonomy_path, pending_path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagMeta {
    pub aliases: Vec<String>,
    pub category: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTag {
    pub proposed_at: String,
    pub file_count: u32,
    pub example_files: Vec<String>,
    pub ai_context: String,
}

pub type Taxonomy = HashMap<String, TagMeta>;
pub type Pending = HashMap<String, PendingTag>;

// ── Load / Save ───────────────────────────────────────────────────────────────

pub fn load_taxonomy() -> Taxonomy {
    let path = taxonomy_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Taxonomy::new()
    }
}

pub fn save_taxonomy(tax: &Taxonomy) -> Result<()> {
    let path = taxonomy_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, serde_json::to_string_pretty(tax)?)?;
    Ok(())
}

pub fn load_pending() -> Pending {
    let path = pending_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Pending::new()
    }
}

pub fn save_pending(pending: &Pending) -> Result<()> {
    let path = pending_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, serde_json::to_string_pretty(pending)?)?;
    Ok(())
}

// ── Tag operations ────────────────────────────────────────────────────────────

pub fn approved_tags(tax: &Taxonomy) -> Vec<String> {
    let mut tags: Vec<String> = tax.keys().cloned().collect();
    for meta in tax.values() {
        for alias in &meta.aliases {
            tags.push(alias.clone());
        }
    }
    tags.sort();
    tags.dedup();
    tags
}

pub fn normalize_tag(tag: &str, tax: &Taxonomy) -> Option<String> {
    let tag_lower = tag.to_lowercase();
    if tax.contains_key(&tag_lower) {
        return Some(tag_lower);
    }
    for (canonical, meta) in tax {
        if meta.aliases.iter().any(|a| a.to_lowercase() == tag_lower) {
            return Some(canonical.clone());
        }
    }
    None
}

/// Split AI tags into (approved, new_tags_for_pending_queue)
pub fn resolve_tags(
    ai_tags: &[String],
    filename: &str,
    tax: &Taxonomy,
    pending: &mut Pending,
) -> (Vec<String>, Vec<String>) {
    let mut approved = Vec::new();
    let mut new_tags = Vec::new();

    for tag in ai_tags {
        if let Some(canonical) = normalize_tag(tag, tax) {
            if !approved.contains(&canonical) {
                approved.push(canonical);
            }
        } else {
            new_tags.push(tag.clone());
            // Add to pending queue
            let entry = pending.entry(tag.clone()).or_insert_with(|| PendingTag {
                proposed_at: chrono::Utc::now().to_rfc3339(),
                file_count: 0,
                example_files: Vec::new(),
                ai_context: String::new(),
            });
            entry.file_count += 1;
            if !entry.example_files.contains(&filename.to_string()) {
                entry.example_files.push(filename.to_string());
                entry.example_files.truncate(5);
            }
        }
    }

    (approved, new_tags)
}

pub fn add_tag(tag: &str, category: &str, aliases: Vec<String>) -> Result<()> {
    let mut tax = load_taxonomy();
    tax.insert(tag.to_lowercase(), TagMeta {
        aliases: aliases.into_iter().map(|a| a.to_lowercase()).collect(),
        category: category.to_string(),
        added_at: chrono::Utc::now().to_rfc3339(),
    });
    save_taxonomy(&tax)
}

pub fn remove_tag(tag: &str) -> Result<()> {
    let mut tax = load_taxonomy();
    tax.remove(&tag.to_lowercase());
    save_taxonomy(&tax)
}

pub fn approve_pending(tag: &str, category: &str) -> Result<()> {
    let mut pending = load_pending();
    if pending.remove(tag).is_some() {
        add_tag(tag, category, vec![])?;
        save_pending(&pending)?;
    }
    Ok(())
}

pub fn reject_pending(tag: &str) -> Result<()> {
    let mut pending = load_pending();
    pending.remove(tag);
    save_pending(&pending)
}

pub fn merge_pending(tag: &str, into: &str) -> Result<()> {
    let mut pending = load_pending();
    if pending.remove(tag).is_some() {
        let mut tax = load_taxonomy();
        if let Some(meta) = tax.get_mut(into) {
            if !meta.aliases.contains(&tag.to_lowercase()) {
                meta.aliases.push(tag.to_lowercase());
            }
        }
        save_taxonomy(&tax)?;
        save_pending(&pending)?;
    }
    Ok(())
}

pub fn pending_count() -> usize {
    load_pending().len()
}

// ── Default taxonomy seed ─────────────────────────────────────────────────────

pub fn init_taxonomy() -> Result<()> {
    if taxonomy_path().exists() {
        return Ok(());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let defaults: Vec<(&str, &str, Vec<&str>)> = vec![
        ("work",          "work",      vec!["professional", "job", "office"]),
        ("finance",       "work",      vec!["budget", "money", "invoice", "billing", "tax", "expense"]),
        ("legal",         "work",      vec!["contract", "agreement", "law"]),
        ("report",        "work",      vec!["analysis", "summary", "review"]),
        ("project",       "work",      vec!["plan", "roadmap", "milestone"]),
        ("presentation",  "work",      vec!["slides", "deck", "pptx"]),
        ("spreadsheet",   "work",      vec!["excel", "xlsx", "data"]),
        ("resume",        "work",      vec!["cv", "curriculum-vitae"]),
        ("email",         "work",      vec!["correspondence", "letter", "memo"]),
        ("meeting",       "work",      vec!["minutes", "agenda", "notes"]),
        ("education",     "education", vec!["school", "university", "course", "class"]),
        ("research",      "education", vec!["paper", "study", "thesis", "dissertation"]),
        ("notes",         "education", vec!["lecture", "study-notes", "notebook"]),
        ("assignment",    "education", vec!["homework", "essay", "exam"]),
        ("code",          "technical", vec!["programming", "software", "script", "dev"]),
        ("config",        "technical", vec!["configuration", "settings", "dotfile"]),
        ("documentation", "technical", vec!["docs", "readme", "manual", "guide"]),
        ("database",      "technical", vec!["sql", "db"]),
        ("infrastructure","technical", vec!["devops", "server", "cloud", "docker"]),
        ("security",      "technical", vec!["cybersecurity", "pentest", "vulnerability", "soc"]),
        ("ai",            "technical", vec!["machine-learning", "ml", "llm", "model"]),
        ("personal",      "personal",  vec!["private", "home"]),
        ("medical",       "personal",  vec!["health", "doctor", "prescription"]),
        ("travel",        "personal",  vec!["vacation", "trip", "itinerary"]),
        ("photo",         "personal",  vec!["image", "picture", "photography"]),
        ("video",         "personal",  vec!["movie", "film", "recording"]),
        ("music",         "personal",  vec!["audio", "song", "album"]),
        ("receipt",       "personal",  vec!["purchase", "order", "transaction"]),
        ("military",      "military",  vec!["army", "dod", "defense", "armed-forces"]),
        ("government",    "military",  vec!["federal", "agency", "nasa", "official"]),
        ("training",      "military",  vec!["exercise", "drill", "instruction"]),
        ("operations",    "military",  vec!["ops", "mission", "deployment"]),
        ("intelligence",  "military",  vec!["intel", "assessment", "brief"]),
        ("archive",       "misc",      vec!["old", "backup", "archived"]),
        ("template",      "misc",      vec!["form", "blank", "boilerplate"]),
        ("reference",     "misc",      vec!["resource", "reference-material"]),
        ("draft",         "misc",      vec!["wip", "work-in-progress"]),
    ];

    let mut tax = Taxonomy::new();
    for (tag, category, aliases) in defaults {
        tax.insert(tag.to_string(), TagMeta {
            aliases: aliases.into_iter().map(String::from).collect(),
            category: category.to_string(),
            added_at: now.clone(),
        });
    }

    save_taxonomy(&tax)
}
