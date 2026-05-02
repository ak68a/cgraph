use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::model::Language;

pub fn detect_language(path: &Path) -> Option<Language> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("ts") => Some(Language::TypeScript),
        Some("tsx") => Some(Language::TypeScriptReact),
        Some("swift") => Some(Language::Swift),
        Some("go") => Some(Language::Go),
        Some("py") => Some(Language::Python),
        Some(ext) => Some(Language::Unknown(ext.to_string())),
        None => None,
    }
}

pub fn is_parseable(lang: &Language) -> bool {
    matches!(
        lang,
        Language::TypeScript
            | Language::TypeScriptReact
            | Language::Swift
            | Language::Go
            | Language::Python
    )
}

#[derive(Debug, Default)]
pub struct DetectionResult {
    pub detected: Vec<(PathBuf, Language)>,
    pub parseable: Vec<(PathBuf, Language)>,
    pub skipped: Vec<(PathBuf, String)>,
}

pub fn scan_directory(path: &Path) -> Result<DetectionResult, std::io::Error> {
    let mut result = DetectionResult::default();
    let mut it = WalkDir::new(path).follow_links(false).into_iter();

    loop {
        let entry = match it.next() {
            None => break,
            Some(Err(_)) => continue,
            Some(Ok(e)) => e,
        };

        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy();
            if name.starts_with('.')
                || name == "node_modules"
                || name == "dist"
                || name == "build"
                || name == "target"
            {
                it.skip_current_dir();
            }
            continue;
        }

        if !entry.file_type().is_file() {
            continue;
        }

        let file_path = entry.path().to_path_buf();
        match detect_language(&file_path) {
            None => {}
            Some(lang) => {
                if is_parseable(&lang) {
                    result.parseable.push((file_path.clone(), lang.clone()));
                } else if let Language::Unknown(ref ext) = lang {
                    result.skipped.push((file_path.clone(), ext.clone()));
                }
                result.detected.push((file_path, lang));
            }
        }
    }

    Ok(result)
}
