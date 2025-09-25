use std::{fs::metadata, os::unix::fs::MetadataExt, path::PathBuf};

use crate::path_utils::ExpandPaths;

/// Validates a directory
///
/// Expands path and checks if it exists and is a directory.
///
/// Used in `sericom`s `clap` cli interface to validate user input and
/// propogate errors before running the program.
///
/// # Errors
/// Errors if path-expansion fails or if the path doesn't exist or is not a dir.
pub fn validate_dir(input: &str) -> Result<Option<PathBuf>, String> {
    let p = PathBuf::from(input)
        .get_expanded_path()
        .ok_or("Error expanding path.")?;
    if !p.exists() || !p.is_dir() {
        return Err(format!(
            "Invalid directory '{input}' out-dir\nEither does not exist or is not a directory"
        ));
    }
    Ok(Some(p))
}

/// Validates a script
///
/// Expands path and checks if it exists and is a file. Then checks whether
/// the file is executable.
///
/// ## Unix
/// On Unix this checks whether there are any executable bits in the file's
/// permissions (i.e. '-rwxrw--x')
///
/// ## Windows
/// On Windows this just checks whether the file ends in '.exe', '.bat',
/// '.cmd', '.com', or '.ps1'.
///
/// Used in `sericom`s `clap` cli interface to validate user input and
/// propogate errors before running the program.
///
/// # Errors
/// Errors if path-expansion fails, if the path doesn't exist, is not a file,
/// or is not executable according to the definitions in [Unix](##Unix) and [Windows](##Windows)
/// respectively.
pub fn is_script(input: &str) -> Result<Option<PathBuf>, String> {
    let p = PathBuf::from(input)
        .get_expanded_path()
        .ok_or("Error expanding path.")?;

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
        mode & 0o111 != 0
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
