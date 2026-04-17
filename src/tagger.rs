use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize)]
struct TagResponse {
    summary: String,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

fn build_system_prompt(approved_tags: &[String]) -> String {
    let tag_section = if !approved_tags.is_empty() {
        let tag_list = approved_tags.iter()
            .take(150)
            .map(|t| format!("  - {t}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(r#"
APPROVED TAG LIBRARY (use these whenever possible):
{tag_list}

TAG SELECTION RULES — FOLLOW STRICTLY:
1. ALWAYS prefer tags from the approved library above.
2. Only invent a NEW tag if NO approved tag adequately describes the file.
3. If you must invent a new tag, limit it to 1 new tag maximum per file.
4. New tags must be BROAD and REUSABLE (e.g. "geology" not "utah-rock-formations-2019").
5. Never create tags specific to a single file (filenames, project codes, dates).
6. Favor FEWER, MORE GENERAL tags. 3-5 tags is ideal."#)
    } else {
        r#"
TAG SELECTION RULES:
1. Generate 3-6 broad, reusable tags describing the file's topic and type.
2. Tags must be general enough to apply to multiple files.
3. Never use filenames, dates, or unique identifiers as tags."#.to_string()
    };

    format!(r#"You are a file tagging assistant. Given file content and metadata, generate:
1. A concise one-sentence summary (max 20 words)
2. A list of 3-6 relevant tags
{tag_section}

Respond ONLY with valid JSON in this exact format, no other text:
{{"summary": "...", "tags": ["tag1", "tag2", "tag3"]}}"#)
}

fn build_user_prompt(
    filename: &str,
    category: &str,
    extension: &str,
    content: &str,
    size_bytes: i64,
) -> String {
    let size_kb = size_bytes / 1024;
    let content_section = if !content.is_empty() {
        format!("\n\nFILE CONTENT (first {} chars):\n{}", content.len(), content)
    } else {
        String::new()
    };
    format!(
        "FILE: {filename}\nTYPE: {category} ({extension})\nSIZE: {size_kb} KB{content_section}\n\nGenerate summary and tags for this file."
    )
}

pub async fn tag_file(
    client: &reqwest::Client,
    base_url: &str,
    model: &str,
    filename: &str,
    category: &str,
    extension: &str,
    content: &str,
    size_bytes: i64,
    approved_tags: &[String],
) -> Result<(String, Vec<String>)> {
    let system = build_system_prompt(approved_tags);
    let user = build_user_prompt(filename, category, extension, content, size_bytes);

    let payload = OllamaRequest {
        model: model.to_string(),
        messages: vec![
            OllamaMessage { role: "system".into(), content: system },
            OllamaMessage { role: "user".into(), content: user },
        ],
        stream: false,
        options: OllamaOptions { temperature: 0.2, num_predict: 256 },
    };

    let url = format!("{}/api/chat", base_url.trim_end_matches('/'));
    let resp = client.post(&url)
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Ollama HTTP {status}: {body}"));
    }

    let data: OllamaResponse = resp.json().await?;
    let mut raw = data.message.content.trim().to_string();

    // Strip markdown code fences
    if raw.starts_with("```") {
        let parts: Vec<&str> = raw.splitn(3, "```").collect();
        if parts.len() >= 2 {
            raw = parts[1].to_string();
            if raw.starts_with("json") {
                raw = raw[4..].to_string();
            }
            raw = raw.trim().to_string();
        }
    }

    let parsed: TagResponse = serde_json::from_str(&raw)
        .map_err(|e| anyhow!("JSON parse failed ({e}): {raw:.200}"))?;

    let tags: Vec<String> = parsed.tags.into_iter()
        .map(|t| t.to_lowercase().trim().replace(' ', "-").chars().take(40).collect::<String>())
        .filter(|t| !t.is_empty())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .take(10)
        .collect();

    Ok((parsed.summary, tags))
}

pub async fn check_ollama(client: &reqwest::Client, base_url: &str, model: &str) -> Result<String> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let resp = client.get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| anyhow!("Cannot connect to Ollama at {base_url}: {e}"))?;

    let data: OllamaTagsResponse = resp.json().await?;
    let model_base = model.split(':').next().unwrap_or(model);
    let available: Vec<String> = data.models.iter()
        .map(|m| m.name.split(':').next().unwrap_or(&m.name).to_string())
        .collect();

    if !available.iter().any(|m| m == model_base) {
        return Err(anyhow!(
            "Model '{model}' not found. Available: {}",
            available.join(", ")
        ));
    }

    Ok(format!("Connected to {base_url}, model '{model}' available"))
}
