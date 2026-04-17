use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Extract text content from a file for AI tagging.
/// Returns empty string if extraction fails or is unsupported.
pub fn extract(path: &Path, category: &str, max_chars: usize, ocr_enabled: bool) -> String {
    let result = match category {
        "documents" => extract_document(path, max_chars),
        "images"    => if ocr_enabled { extract_image_ocr(path, max_chars) } else { Ok(String::new()) },
        "code"      => read_text(path, max_chars),
        "audio"     => Ok(format!("Audio file: {}", path.file_name().unwrap_or_default().to_string_lossy())),
        "video"     => Ok(format!("Video file: {}", path.file_name().unwrap_or_default().to_string_lossy())),
        _           => read_text(path, max_chars),
    };
    result.unwrap_or_default()
}

fn extract_document(path: &Path, max_chars: usize) -> Result<String> {
    let ext = path.extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default();

    match ext.as_str() {
        ".pdf"              => extract_pdf(path, max_chars),
        ".doc" | ".docx"   => extract_docx(path, max_chars),
        ".odt"             => extract_odt(path, max_chars),
        ".xlsx" | ".xls"   => extract_xlsx(path, max_chars),
        ".pptx" | ".ppt"   => extract_pptx(path, max_chars),
        ".txt" | ".md" | ".rtf" | ".csv" => read_text(path, max_chars),
        _                  => read_text(path, max_chars),
    }
}

fn extract_pdf(path: &Path, max_chars: usize) -> Result<String> {
    // Try pdf-extract crate first
    match pdf_extract::extract_text(path) {
        Ok(text) if !text.trim().is_empty() => {
            return Ok(text.chars().take(max_chars).collect());
        }
        _ => {}
    }

    // Fallback: pdftotext CLI (poppler-utils, already in BlueAK image)
    let output = Command::new("pdftotext")
        .args([path.to_str().unwrap_or(""), "-"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            Ok(text.chars().take(max_chars).collect())
        }
        _ => Ok(String::new()),
    }
}

fn extract_docx(path: &Path, max_chars: usize) -> Result<String> {
    use std::io::Read;

    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // word/document.xml contains the text
    let mut xml_content = String::new();
    let mut xml_file = archive.by_name("word/document.xml")?;
    xml_file.read_to_string(&mut xml_content)?;

    Ok(extract_xml_text(&xml_content, max_chars))
}

fn extract_odt(path: &Path, max_chars: usize) -> Result<String> {
    // Try odt2txt CLI first (installed in BlueAK)
    let output = Command::new("odt2txt")
        .arg(path.to_str().unwrap_or(""))
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            return Ok(text.chars().take(max_chars).collect());
        }
    }

    // Fallback: extract content.xml from the zip
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut xml_content = String::new();
    let mut xml_file = archive.by_name("content.xml")?;
    xml_file.read_to_string(&mut xml_content)?;
    Ok(extract_xml_text(&xml_content, max_chars))
}

fn extract_xlsx(path: &Path, max_chars: usize) -> Result<String> {
    use calamine::{open_workbook_auto, Reader};

    let mut workbook = open_workbook_auto(path)?;
    let mut texts = Vec::new();

    for sheet_name in workbook.sheet_names().to_owned() {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            for row in range.rows() {
                let line: Vec<String> = row.iter()
                    .filter(|c| !matches!(c, calamine::Data::Empty))
                    .map(|c| c.to_string())
                    .collect();
                if !line.is_empty() {
                    texts.push(line.join(" "));
                }
            }
        }
    }

    Ok(texts.join("\n").chars().take(max_chars).collect())
}

fn extract_pptx(path: &Path, max_chars: usize) -> Result<String> {
    use std::io::Read;

    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut texts = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            texts.push(extract_xml_text(&content, max_chars));
        }
    }

    Ok(texts.join("\n").chars().take(max_chars).collect())
}

fn extract_xml_text(xml: &str, max_chars: usize) -> String {
    // Extract text nodes from XML, stripping tags
    let doc = roxmltree::Document::parse(xml).ok();
    let Some(doc) = doc else {
        // Fallback: naive tag stripping
        return strip_xml_tags(xml, max_chars);
    };

    let text: String = doc.descendants()
        .filter(|n| n.is_text())
        .map(|n| n.text().unwrap_or(""))
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    text.chars().take(max_chars).collect()
}

fn strip_xml_tags(xml: &str, max_chars: usize) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in xml.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => result.push(c),
            _ => {}
        }
    }
    result.chars().take(max_chars).collect()
}

fn extract_image_ocr(path: &Path, max_chars: usize) -> Result<String> {
    // Shell out to tesseract (available in BlueAK image)
    let output = Command::new("tesseract")
        .args([path.to_str().unwrap_or(""), "stdout", "--psm", "3"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            Ok(text.chars().take(max_chars).collect())
        }
        _ => Ok(String::new()),
    }
}

fn read_text(path: &Path, max_chars: usize) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .unwrap_or_default();
    Ok(content.chars().take(max_chars).collect())
}
