use crate::app::VenvMetadata;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn find_venvs<P: AsRef<std::path::Path>>(search_path: P) -> Vec<PathBuf> {
    let mut venvs = Vec::new();

    let mut it = WalkDir::new(search_path).follow_links(false).into_iter();

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
    meta.packages = get_installed_packages(venv_path);
    Some(meta)
}

pub fn get_installed_packages(venv_path: &std::path::Path) -> Vec<String> {
    let mut packages = Vec::new();
    let lib_dir = venv_path.join("lib");

    if let Ok(entries) = fs::read_dir(lib_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("python"))
            {
                let site_packages = path.join("site-packages");
                if let Ok(pkg_entries) = fs::read_dir(site_packages) {
                    for pkg in pkg_entries.flatten() {
                        let name = pkg.file_name().to_string_lossy().into_owned();
                        if name.ends_with(".dist-info") {
                            let raw_name = &name[..name.len() - 10];
                            if let Some((pkg_name, version)) = raw_name.rsplit_once('-') {
                                packages.push(format!("{} == {}", pkg_name, version));
                            } else {
                                packages.push(raw_name.to_string());
                            }
                        }
                    }
                }
                break;
            }
        }
    }
    packages.sort();
    packages
}
