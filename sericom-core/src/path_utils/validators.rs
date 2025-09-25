use std::{fs::metadata, os::unix::fs::MetadataExt, path::PathBuf};

use crate::path_utils::ExpandUnixPaths;

pub fn validate_dir(input: &str) -> Result<Option<PathBuf>, String> {
    let p = PathBuf::from(input)
        .get_expanded_path()
        .map_err(|_| "Error expanding path.")?;
    if !p.exists() || !p.is_dir() {
        return Err(format!(
            "Invalid directory '{input}' out-dir\nEither does not exist or is not a directory"
        ));
    }
    Ok(Some(p))
}

pub fn is_script(input: &str) -> Result<Option<PathBuf>, String> {
    let p = PathBuf::from(input)
        .get_expanded_path()
        .map_err(|_| "Error expanding path.")?;

    if !p.exists() || !p.is_file() {
        return Err(format!(
            "Invalid file '{input}'\nEither does not exist or is not a file"
        ));
    }

    if !is_executable(&p) {
        return Err(format!(
            "Invalid file type '{}'\nMake sure the file is executable",
            p.display()
        ));
    }

    Ok(Some(p))
}

pub(crate) fn is_executable(path: &std::path::Path) -> bool {
    #[cfg(unix)]
    {
        let Ok(script) = metadata(path) else {
            return false;
        };
        let mode = script.mode();
        mode & 0o111 == 0
    }

    #[cfg(windows)]
    {
        match path.extension() {
            Some(ext) => matches!(
                ext.to_ascii_lowercase()
                    .to_str()
                    .expect("Converted to ascii"),
                "exe" | "bat" | "cmd" | "com" | "ps1"
            ),
            None => false,
        }
    }
}
