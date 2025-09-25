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

    let script = metadata(&p).map_err(|p| {
        // Only error would be if permission issues
        p.to_string()
    })?;

    if !cfg!(windows) {
        let mode = script.mode();
        if mode & 0o111 == 0 {
            return Err(format!(
                "Invalid file type '{input}'\nMake sure the file is executable"
            ));
        }
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

    let script = metadata(&p).map_err(|p| {
        // Only error would be if permission issues
        p.to_string()
    })?;

    if !cfg!(windows) {
        let mode = script.mode();
        if mode & 0o111 == 0 {
            return Err(format!(
                "Invalid file type '{input}'\nMake sure the file is executable"
            ));
        }
    }

    Ok(Some(p))
}
