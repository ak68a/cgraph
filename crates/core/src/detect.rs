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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // --- detect_language unit tests ---

    #[test]
    fn test_detect_ts() {
        assert_eq!(detect_language(Path::new("foo.ts")), Some(Language::TypeScript));
    }

    #[test]
    fn test_detect_tsx() {
        assert_eq!(detect_language(Path::new("foo.tsx")), Some(Language::TypeScriptReact));
    }

    #[test]
    fn test_detect_swift() {
        assert_eq!(detect_language(Path::new("foo.swift")), Some(Language::Swift));
    }

    #[test]
    fn test_detect_go() {
        assert_eq!(detect_language(Path::new("main.go")), Some(Language::Go));
    }

    #[test]
    fn test_detect_py() {
        assert_eq!(detect_language(Path::new("app.py")), Some(Language::Python));
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(
            detect_language(Path::new("data.json")),
            Some(Language::Unknown("json".to_string()))
        );
    }

    #[test]
    fn test_detect_no_extension() {
        assert_eq!(detect_language(Path::new("Makefile")), None);
    }

    // --- is_parseable unit tests ---

    #[test]
    fn test_is_parseable_known() {
        assert!(is_parseable(&Language::TypeScript));
        assert!(is_parseable(&Language::TypeScriptReact));
        assert!(is_parseable(&Language::Swift));
        assert!(is_parseable(&Language::Go));
        assert!(is_parseable(&Language::Python));
    }

    #[test]
    fn test_is_parseable_unknown() {
        assert!(!is_parseable(&Language::Unknown("json".into())));
    }

    // --- scan_directory tests ---

    #[test]
    fn test_scan_directory() {
        let tmp = std::env::temp_dir().join("cgraph_test_scan_directory");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::create_dir_all(tmp.join("src")).unwrap();

        // Create mixed files
        std::fs::write(tmp.join("src/main.ts"), "const x = 1;").unwrap();
        std::fs::write(tmp.join("src/App.tsx"), "export default () => <div/>;").unwrap();
        std::fs::write(tmp.join("src/main.go"), "package main").unwrap();
        std::fs::write(tmp.join("src/app.py"), "def main(): pass").unwrap();
        std::fs::write(tmp.join("src/view.swift"), "import Foundation").unwrap();
        std::fs::write(tmp.join("config.json"), "{}").unwrap();
        std::fs::write(tmp.join("Makefile"), "build:").unwrap();

        let result = scan_directory(&tmp).unwrap();

        // 6 files with extensions detected (Makefile has no extension, silently skipped)
        assert_eq!(result.detected.len(), 6, "detected count mismatch");
        // 5 parseable (ts, tsx, go, py, swift)
        assert_eq!(result.parseable.len(), 5, "parseable count mismatch");
        // 1 skipped (json)
        assert_eq!(result.skipped.len(), 1, "skipped count mismatch");
        assert_eq!(result.skipped[0].1, "json");

        // Cleanup
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_scan_skips_hidden_dirs() {
        let tmp = std::env::temp_dir().join("cgraph_test_scan_hidden");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::create_dir_all(tmp.join(".git")).unwrap();

        std::fs::write(tmp.join(".git/foo.ts"), "const x = 1;").unwrap();
        std::fs::write(tmp.join("real.ts"), "export const y = 2;").unwrap();

        let result = scan_directory(&tmp).unwrap();

        // Only real.ts should be detected; .git/foo.ts should be skipped
        assert_eq!(result.detected.len(), 1, "hidden dir file should be excluded");
        assert!(result.detected[0].0.ends_with("real.ts"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_scan_skips_node_modules() {
        let tmp = std::env::temp_dir().join("cgraph_test_scan_node_modules");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::create_dir_all(tmp.join("node_modules")).unwrap();

        std::fs::write(tmp.join("node_modules/bar.ts"), "const x = 1;").unwrap();
        std::fs::write(tmp.join("index.ts"), "export const y = 2;").unwrap();

        let result = scan_directory(&tmp).unwrap();

        // Only index.ts should be detected; node_modules/bar.ts should be skipped
        assert_eq!(result.detected.len(), 1, "node_modules file should be excluded");
        assert!(result.detected[0].0.ends_with("index.ts"));

        std::fs::remove_dir_all(&tmp).ok();
    }
}
