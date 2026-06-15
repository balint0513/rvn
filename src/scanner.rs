use std::path::{PathBuf};
use walkdir::WalkDir;
use crate::app::VenvMetadata;
use std::fs;

pub fn find_venvs<P: AsRef<std::path::Path>>(search_path: P) -> Vec<PathBuf> {
    let mut venvs = Vec::new();

    let mut it = WalkDir::new(search_path)
        .follow_links(false)
        .into_iter();

    while let Some(entry) = it.next() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.file_type().is_dir() {
            let path = entry.path();
            let config_file = path.join("pyvenv.cfg");

            if config_file.is_file() {
                venvs.push(path.to_path_buf());
                it.skip_current_dir();
            }
        }
    }

    venvs

}

pub fn parse_venv_metadata(venv_path: &std::path::Path) -> Option<VenvMetadata> {
    let config_path = venv_path.join("pyvenv.cfg");
    let content = fs::read_to_string(config_path).ok()?;

    let mut meta = VenvMetadata::default();

    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "version" => meta.version = value.trim().to_string(),
                "executable" => meta.executable = value.trim().to_string(),
                "include-system-site-packages" => {
                    meta.include_system_packages = value.trim().to_string()
                }
                _ => {}
            }
        }
    }
    Some(meta)
}
