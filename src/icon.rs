use image::DynamicImage;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct IconData {
    pub width: u32,
    pub height: u32,
    pub rgba: Arc<Vec<u8>>,
}

impl IconData {
    pub fn from_dynamic(img: &DynamicImage) -> Self {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Self {
            width: w,
            height: h,
            rgba: Arc::new(rgba.into_raw()),
        }
    }
}

#[derive(Default)]
pub struct IconCache {
    by_ext: HashMap<String, IconData>,
    by_path: HashMap<String, IconData>,
}

impl IconCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_ext(&self, ext: &str) -> Option<&IconData> {
        self.by_ext.get(ext)
    }

    pub fn get_path(&self, path: &str) -> Option<&IconData> {
        self.by_path.get(path)
    }

    pub fn insert_ext(&mut self, ext: String, icon: IconData) {
        self.by_ext.insert(ext, icon);
    }

    pub fn insert_path(&mut self, path: String, icon: IconData) {
        self.by_path.insert(path, icon);
    }
}

const SEARCH_DIRS: &[&str] = &[
    "/usr/share/icons/Adwaita/48x48/mimetypes",
    "/usr/share/icons/Adwaita/48x48/status",
    "/usr/share/icons/gnome/48x48/mimetypes",
    "/usr/share/icons/gnome/48x48/actions",
    "/usr/share/icons/hicolor/48x48/mimetypes",
    "/usr/share/icons/hicolor/32x32/mimetypes",
    "/usr/share/icons/hicolor/24x24/mimetypes",
    "/usr/share/icons/hicolor/16x16/mimetypes",
];

fn find_icon_path(ext: &str) -> Option<std::path::PathBuf> {
    let ext_lower = ext.trim_start_matches('.').to_lowercase();

    let name_map: &[(&str, &str)] = &[
        (".pdf", "application-pdf"),
        (".doc", "application-msword"),
        (".docx", "application-vnd.openxmlformats-officedocument.wordprocessingml.document"),
        (".txt", "text-plain"),
        (".md", "text-markdown"),
        (".xls", "application-vnd.ms-excel"),
        (".xlsx", "application-vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        (".ppt", "application-vnd.ms-powerpoint"),
        (".pptx", "application-vnd.openxmlformats-officedocument.presentationml.presentation"),
        (".py", "text-x-python"),
        (".js", "text-javascript"),
        (".ts", "text-ts"),
        (".sh", "text-x-shellscript"),
        (".bash", "text-x-shellscript"),
        (".json", "application-json"),
        (".yaml", "application-yaml"),
        (".yml", "application-yaml"),
        (".jpg", "image-jpeg"),
        (".jpeg", "image-jpeg"),
        (".png", "image-png"),
        (".gif", "image-gif"),
        (".webp", "image-webp"),
        (".mp3", "audio-mpeg"),
        (".wav", "audio-x-wav"),
        (".flac", "audio-flac"),
        (".mp4", "video-mp4"),
        (".mkv", "video-x-matroska"),
        (".avi", "video-x-msvideo"),
        (".zip", "application-zip"),
        (".tar", "application-tar"),
        (".gz", "application-gzip"),
        (".rar", "application-vnd.rar"),
        (".svg", "image-svg+xml"),
    ];

    let icon_name = name_map.iter()
        .find(|(e, _)| *e == ext_lower)
        .map(|(_, n)| *n)
        .unwrap_or("text-x-generic");

    for dir in SEARCH_DIRS {
        let base = std::path::PathBuf::from(dir);
        if !base.exists() {
            continue;
        }
        for file_ext in &["png", "svg", "xpm"] {
            let path = base.join(format!("{}.{}", icon_name, file_ext));
            if path.exists() {
                return Some(path);
            }
        }

        let alt_name = match ext_lower.as_str() {
            ".docx" => "x-office-document",
            ".xlsx" => "x-office-spreadsheet",
            ".pptx" => "x-office-presentation",
            ".jpg" | ".jpeg" | ".png" | ".gif" | ".webp" => "image-x-generic",
            ".mp3" => "audio-x-generic",
            ".mp4" => "video-x-generic",
            ".mkv" => "video-x-generic",
            ".zip" | ".tar" | ".gz" => "package-x-generic",
            ".svg" => "text-html",
            _ => "text-x-generic",
        };

        if alt_name != icon_name {
            let path = base.join(format!("{}.png", alt_name));
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

pub fn load_icon_for_ext(ext: &str) -> Option<IconData> {
    let path = find_icon_path(ext)?;
    load_png_icon(&path)
}

pub fn load_thumbnail_for_path(path: &str, size: u32) -> Option<IconData> {
    let img = image::open(path).ok()?;
    let thumb = img.thumbnail(size, size);
    Some(IconData::from_dynamic(&thumb))
}

fn load_png_icon(path: &std::path::Path) -> Option<IconData> {
    let img = image::open(path).ok()?;
    Some(IconData::from_dynamic(&img))
}

pub fn is_image_file(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        ".jpg" | ".jpeg" | ".png" | ".gif" | ".webp" | ".bmp" | ".ico"
    )
}

pub fn is_svg_file(ext: &str) -> bool {
    ext.to_lowercase() == ".svg"
}
