use crate::error::Result;
use crate::storage::database::Database;

/// Case-insensitive substring search across title, artist, album.
/// Returns track ids matching the query.
#[allow(dead_code)]
pub fn search_library(db: &Database, query: &str) -> Result<Vec<i64>> {
    let pattern = format!("%{}%", query.to_lowercase());
    db.search_tracks(&pattern)
}
