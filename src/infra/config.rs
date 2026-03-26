use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::domain::filter::FilterFile;

/// Default directory name under the user's config home.
const APP_DIR: &str = "purectx";
/// Subdirectory where custom filter files are stored.
const FILTERS_DIR: &str = "filters";

/// Return the path to the custom filters directory
/// (`~/.config/purectx/filters/` on Linux).
///
/// # Errors
/// Returns an error if the user's config directory cannot be determined.
pub fn filters_dir() -> Result<PathBuf> {
    let config = dirs::config_dir().context("unable to determine config directory")?;
    Ok(config.join(APP_DIR).join(FILTERS_DIR))
}

/// Ensure the custom filters directory exists, creating it if needed.
///
/// # Errors
/// Returns an error if directory creation fails.
pub fn ensure_filters_dir() -> Result<PathBuf> {
    let dir = filters_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create filters directory: {}", dir.display()))?;
    Ok(dir)
}

/// Install a custom filter file into the shared filters directory.
///
/// The file is validated (parsed as TOML) before being copied.
///
/// # Errors
/// Returns an error if the file cannot be read, is not valid TOML, or cannot
/// be written to the target directory.
pub fn add_filter(source_path: &str) -> Result<String> {
    let content =
        fs::read_to_string(source_path).with_context(|| format!("cannot read `{source_path}`"))?;

    let filter = FilterFile::from_toml(&content)
        .with_context(|| format!("invalid filter file `{source_path}`"))?;

    let dir = ensure_filters_dir()?;
    let dest = dir.join(format!("{}.toml", filter.name));
    fs::write(&dest, &content).with_context(|| format!("cannot write to `{}`", dest.display()))?;

    Ok(filter.name)
}

/// Load all custom filters from the shared filters directory.
///
/// Missing directory is not an error — an empty list is returned.
///
/// # Errors
/// Returns an error if a file in the directory cannot be read or parsed.
pub fn load_custom_filters() -> Result<Vec<FilterFile>> {
    let dir = match filters_dir() {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut filters = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("cannot read `{}`", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("cannot read `{}`", path.display()))?;
            let filter = FilterFile::from_toml(&content)
                .with_context(|| format!("invalid filter `{}`", path.display()))?;
            filters.push(filter);
        }
    }

    Ok(filters)
}

/// List all available filters (built-in + custom) with their metadata.
///
/// Returns tuples of `(name, description, source)`.
pub fn list_filters() -> Result<Vec<(String, String, &'static str)>> {
    let mut result = Vec::new();

    // Built-in filters
    let builtins =
        crate::infra::builtin::load_builtin_filters().context("failed to load built-in filters")?;
    for f in builtins {
        result.push((f.name, f.description, "built-in"));
    }

    // Custom filters
    let customs = load_custom_filters().context("failed to load custom filters")?;
    for f in customs {
        result.push((f.name, f.description, "custom"));
    }

    Ok(result)
}
