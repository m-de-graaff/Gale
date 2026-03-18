-- GaleX Package Registry — Initial Schema

CREATE TABLE IF NOT EXISTS packages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    author TEXT NOT NULL DEFAULT '',
    license TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id INTEGER NOT NULL REFERENCES packages(id),
    version TEXT NOT NULL,
    checksum TEXT NOT NULL,
    gale_version TEXT NOT NULL DEFAULT '',
    dependencies TEXT NOT NULL DEFAULT '[]',
    tarball_path TEXT NOT NULL,
    published_at TEXT NOT NULL DEFAULT (datetime('now')),
    yanked INTEGER NOT NULL DEFAULT 0,
    UNIQUE(package_id, version)
);

CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    token_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Full-text search index
CREATE VIRTUAL TABLE IF NOT EXISTS package_search USING fts5(name, description);
