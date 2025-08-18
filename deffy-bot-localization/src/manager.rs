use once_cell::sync::Lazy;
use serde::Deserialize;
use std::sync::RwLock;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct LanguageMap(pub HashMap<String, String>);

#[derive(Debug, Default)]
pub struct TranslationManager {
    langs: HashMap<String, LanguageMap>,
    fallback: String,
}

impl TranslationManager {
    pub fn new(fallback: &str) -> Self {
        Self {
            langs: HashMap::new(),
            fallback: fallback.to_string(),
        }
    }

    /// โหลดไฟล์ JSON จากโฟลเดอร์
    pub fn load_from_dir(&mut self, dir: &str) -> anyhow::Result<()> {
        let paths = fs::read_dir(dir)?;
        for entry in paths {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let content = fs::read_to_string(&path)?;
                    let map: HashMap<String, String> = serde_json::from_str(&content)?;
                    self.langs.insert(stem.to_string(), LanguageMap(map));
                }
            }
        }
        Ok(())
    }

    /// ดึงข้อความตาม locale + fallback
    pub fn tr(&self, locale: &str, key: &str) -> String {
        // 1. locale ตรง เช่น "th-TH"
        if let Some(lang) = self.langs.get(locale) {
            if let Some(val) = lang.0.get(key) {
                return val.to_string();
            }
        }

        // 2. ตัด region เช่น "th-TH" -> "th"
        if let Some((lang_code, _region)) = locale.split_once('-') {
            if let Some(lang) = self.langs.get(lang_code) {
                if let Some(val) = lang.0.get(key) {
                    return val.to_string();
                }
            }
        }

        // 3. fallback เช่น en-US
        if let Some(lang) = self.langs.get(&self.fallback) {
            if let Some(val) = lang.0.get(key) {
                return val.to_string();
            }
        }

        format!("??{}??", key)
    }
}

/// Global Translation Manager
pub static TRANSLATIONS: Lazy<RwLock<TranslationManager>> = Lazy::new(|| {
    let mut tm = TranslationManager::new("en-US");

    let project_root = env!("CARGO_MANIFEST_DIR"); // ได้ path ไปยัง root ของโปรเจกต์
    let path = Path::new(project_root).parent().unwrap().join(r"data\locales");

    if path.exists() {
        tm.load_from_dir(&path.to_str().unwrap())
        .expect("Failed to load locales");
        RwLock::new(tm)
    } else {
        panic!("Locales directory does not exist: {}", path.display());
    }
});

/// Macro เรียกสั้น ๆ
#[macro_export]
macro_rules! tr {
    ($locale:expr, $key:expr) => {{
        let tm = $crate::manager::TRANSLATIONS.read().unwrap();
        tm.tr($locale, $key)
    }};
}
