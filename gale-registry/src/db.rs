//! SQLite database layer for the registry.

use rusqlite::{params, Connection, Result};

/// Initialize the database with the schema.
pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch(include_str!("../migrations/001_init.sql"))
}

/// Insert a new package (or return existing ID).
pub fn upsert_package(
    conn: &Connection,
    name: &str,
    description: &str,
    author: &str,
    license: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT OR IGNORE INTO packages (name, description, author, license) VALUES (?1, ?2, ?3, ?4)",
        params![name, description, author, license],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM packages WHERE name = ?1",
        params![name],
        |row| row.get(0),
    )?;
    Ok(id)
}

/// Insert a new version for a package.
pub fn insert_version(
    conn: &Connection,
    package_id: i64,
    version: &str,
    checksum: &str,
    gale_version: &str,
    dependencies: &str,
    tarball_path: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO versions (package_id, version, checksum, gale_version, dependencies, tarball_path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![package_id, version, checksum, gale_version, dependencies, tarball_path],
    )?;
    Ok(())
}

/// Get the latest version of a package.
pub fn get_latest_version(conn: &Connection, name: &str) -> Result<Option<VersionRow>> {
    let mut stmt = conn.prepare(
        "SELECT v.version, v.checksum, v.dependencies, v.tarball_path
         FROM versions v
         JOIN packages p ON v.package_id = p.id
         WHERE p.name = ?1 AND v.yanked = 0
         ORDER BY v.published_at DESC
         LIMIT 1",
    )?;
    let result = stmt.query_row(params![name], |row| {
        Ok(VersionRow {
            version: row.get(0)?,
            checksum: row.get(1)?,
            dependencies: row.get(2)?,
            tarball_path: row.get(3)?,
        })
    });
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Search packages by query.
pub fn search_packages(conn: &Connection, query: &str) -> Result<Vec<SearchResult>> {
    let mut stmt = conn.prepare(
        "SELECT p.name, p.description, v.version
         FROM packages p
         LEFT JOIN versions v ON v.package_id = p.id AND v.yanked = 0
         WHERE p.name LIKE '%' || ?1 || '%' OR p.description LIKE '%' || ?1 || '%'
         GROUP BY p.id
         ORDER BY p.name
         LIMIT 50",
    )?;
    let results = stmt.query_map(params![query], |row| {
        Ok(SearchResult {
            name: row.get(0)?,
            description: row.get(1)?,
            version: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        })
    })?;
    results.collect()
}

/// Verify an auth token. Returns the username if valid.
pub fn verify_token(conn: &Connection, token_hash: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT username FROM users WHERE token_hash = ?1")?;
    match stmt.query_row(params![token_hash], |row| row.get::<_, String>(0)) {
        Ok(username) => Ok(Some(username)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Update the FTS search index for a package.
pub fn update_search_index(conn: &Connection, name: &str, description: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO package_search (name, description) VALUES (?1, ?2)",
        params![name, description],
    )?;
    Ok(())
}

/// A version row from the database.
pub struct VersionRow {
    pub version: String,
    pub checksum: String,
    pub dependencies: String,
    pub tarball_path: String,
}

/// A search result.
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub version: String,
}
