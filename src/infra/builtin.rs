use crate::domain::filter::{FilterError, FilterFile};

/// Built-in filter TOML definitions, embedded at compile time.
const BUILTIN_FILTERS: &[&str] = &[
    include_str!("filters/maven.toml"),
    include_str!("filters/npm.toml"),
    include_str!("filters/cargo.toml"),
    include_str!("filters/dotnet.toml"),
    include_str!("filters/gradle.toml"),
];

/// Load all built-in filters.
///
/// # Errors
/// Returns a [`FilterError`] if any embedded TOML is malformed (should not
/// happen in a correct release).
pub fn load_builtin_filters() -> Result<Vec<FilterFile>, FilterError> {
    BUILTIN_FILTERS
        .iter()
        .map(|toml| FilterFile::from_toml(toml))
        .collect()
}
