//! Asset content hashing and manifest generation for production builds.
//!
//! Provides cache-busting via content-based hashed filenames:
//! `runtime.js` → `runtime.a1b2c3d4.js`
//!
//! The manifest is compiled into the server binary as a static `HashMap`,
//! so asset path resolution is a zero-I/O lookup at runtime.

use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

use super::project::ProjectFiles;
use super::rust_emitter::RustEmitter;

// ── Asset Manifest ─────────────────────────────────────────────────────

/// Maps logical asset paths to their content-hashed filenames.
///
/// Entries use forward-slash paths relative to the `public/` root,
/// e.g. `_gale/runtime.js` → `_gale/runtime.a1b2c3d4.js`.
#[derive(Debug, Clone, Default)]
pub struct AssetManifest {
    /// Logical path → hashed path.
    entries: BTreeMap<String, String>,
}

impl AssetManifest {
    /// Create an empty manifest.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a mapping.
    pub fn insert(&mut self, logical: String, hashed: String) {
        self.entries.insert(logical, hashed);
    }

    /// Look up a hashed path by logical path.
    pub fn resolve(&self, logical: &str) -> Option<&str> {
        self.entries.get(logical).map(|s| s.as_str())
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the manifest is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over `(logical, hashed)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Generate a Rust source module (`src/asset_manifest.rs`) containing
    /// a static `HashMap` and a `resolve()` function.
    pub fn generate_rust_module(&self) -> String {
        let mut e = RustEmitter::new();
        e.emit_file_header("Asset manifest — maps logical paths to content-hashed filenames.");
        e.newline();

        e.emit_use("std::collections::HashMap");
        e.emit_use("std::sync::LazyLock");
        e.newline();

        e.writeln(
            "static MANIFEST: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {",
        );
        e.indent();
        let mutability = if self.entries.is_empty() { "" } else { "mut " };
        e.writeln(&format!(
            "let {mutability}m = HashMap::with_capacity({});",
            self.entries.len()
        ));
        for (logical, hashed) in &self.entries {
            e.writeln(&format!("m.insert({logical:?}, {hashed:?});"));
        }
        e.writeln("m");
        e.dedent();
        e.writeln("});");
        e.newline();

        e.emit_doc_comment("Resolve a logical asset path to its content-hashed filename.");
        e.emit_doc_comment("");
        e.emit_doc_comment(
            "Returns the hashed path if found, otherwise returns the input unchanged.",
        );
        e.block("pub fn resolve(path: &str) -> &str", |e| {
            e.writeln("MANIFEST.get(path).copied().unwrap_or(path)");
        });
        e.newline();

        // Generate import map JSON for browser ES module resolution.
        // This maps `/_gale/runtime.js` → `/_gale/runtime.a1b2c3.js` so that
        // JS `import ... from '/_gale/runtime.js'` resolves to hashed files.
        e.emit_doc_comment("Generate an HTML `<script type=\"importmap\">` tag.");
        e.emit_doc_comment("");
        e.emit_doc_comment("Returns an empty string when there are no hashed assets (dev mode).");
        e.block("pub fn import_map_tag() -> String", |e| {
            e.writeln("if MANIFEST.is_empty() { return String::new(); }");
            e.writeln("let mut json = String::from(\"{\\\"imports\\\":{\");");
            e.writeln("let mut first = true;");
            e.block("for (&logical, &hashed) in MANIFEST.iter()", |e| {
                e.writeln("if !first { json.push(','); }");
                e.writeln("first = false;");
                e.writeln("json.push('\"');");
                e.writeln("json.push('/');");
                e.writeln("json.push_str(logical);");
                e.writeln("json.push_str(\"\\\":\\\"\");");
                e.writeln("json.push('/');");
                e.writeln("json.push_str(hashed);");
                e.writeln("json.push('\"');");
            });
            e.writeln("json.push_str(\"}}\");");
            e.writeln("format!(\"<script type=\\\"importmap\\\">{}</script>\", json)");
        });

        e.finish()
    }

    /// Generate a JSON representation of the manifest (for debugging / external tools).
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n");
        let total = self.entries.len();
        for (i, (logical, hashed)) in self.entries.iter().enumerate() {
            let comma = if i + 1 < total { "," } else { "" };
            let _ = writeln!(out, "  {logical:?}: {hashed:?}{comma}");
        }
        out.push('}');
        out
    }
}

// ── Content Hashing ────────────────────────────────────────────────────

/// Compute a short content hash (8 hex chars) from a byte slice.
///
/// Uses a simple FNV-1a-inspired hash for speed — this is not
/// cryptographic, just a content fingerprint for cache busting.
pub fn hash_content(content: &[u8]) -> String {
    // FNV-1a 64-bit
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in content {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:08x}", h as u32 ^ (h >> 32) as u32)
}

/// Insert a content hash into a filename.
///
/// `runtime.js` + `a1b2c3d4` → `runtime.a1b2c3d4.js`
/// `styles.css` + `deadbeef` → `styles.deadbeef.css`
/// `file`       + `12345678` → `file.12345678`
pub fn insert_hash(filename: &str, hash: &str) -> String {
    match filename.rfind('.') {
        Some(dot) => format!("{}.{hash}.{}", &filename[..dot], &filename[dot + 1..]),
        None => format!("{filename}.{hash}"),
    }
}

// ── Hash & Rename Pipeline ─────────────────────────────────────────────

/// Hash all framework assets in `ProjectFiles` and return the manifest.
///
/// Operates on files under `public/_gale/` — these are the compiler-generated
/// JS/CSS assets. User files from `public/` are left untouched.
///
/// Each matched file is:
/// 1. Content-hashed
/// 2. Renamed in `ProjectFiles` (old key removed, new key inserted)
/// 3. Recorded in the returned manifest
pub fn hash_project_assets(files: &mut ProjectFiles) -> AssetManifest {
    let mut manifest = AssetManifest::new();

    // Collect files to hash (can't mutate while iterating)
    let to_hash: Vec<(PathBuf, String)> = files
        .iter()
        .filter(|(path, _)| {
            let p = path.to_string_lossy();
            p.starts_with("public/_gale/") && (p.ends_with(".js") || p.ends_with(".css"))
        })
        .map(|(path, content)| (path.to_path_buf(), content.to_string()))
        .collect();

    for (path, content) in &to_hash {
        let hash = hash_content(content.as_bytes());

        // Logical path: strip "public/" prefix → "_gale/runtime.js"
        let logical = path
            .strip_prefix("public")
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let logical = logical.trim_start_matches('/').to_string();

        // Extract just the filename, insert hash
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let hashed_filename = insert_hash(&filename, &hash);

        // Build hashed logical path
        let hashed_logical = if let Some(dir) = Path::new(&logical).parent() {
            format!(
                "{}/{hashed_filename}",
                dir.to_string_lossy().replace('\\', "/")
            )
        } else {
            hashed_filename.clone()
        };

        manifest.insert(logical, hashed_logical.clone());

        // Rename in ProjectFiles: remove old, add new with hashed name
        let hashed_path = path.with_file_name(&hashed_filename);
        files.rename(path, &hashed_path);
    }

    manifest
}

/// Copy files from a user's `public/` directory into `ProjectFiles`.
///
/// Files are added under `public/{relative_path}` and are NOT hashed
/// (user controls their own caching strategy).
///
/// Skips dotfiles (`.DS_Store`, `.gitkeep`, etc.).
pub fn copy_public_dir(project_dir: &Path, files: &mut ProjectFiles) -> std::io::Result<usize> {
    let public_dir = project_dir.join("public");
    if !public_dir.is_dir() {
        return Ok(0);
    }

    let mut count = 0;
    copy_dir_recursive(&public_dir, &public_dir, files, &mut count)?;
    Ok(count)
}

fn copy_dir_recursive(
    base: &Path,
    dir: &Path,
    files: &mut ProjectFiles,
    count: &mut usize,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip dotfiles
        if name_str.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            copy_dir_recursive(base, &path, files, count)?;
        } else if path.is_file() {
            let relative = path.strip_prefix(base).unwrap_or(&path);
            let dest = Path::new("public").join(relative);

            // Read as UTF-8 text; skip binary files that fail UTF-8 decode
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    files.add_file(dest, content);
                    *count += 1;
                }
                Err(_) => {
                    // Binary file — read as bytes and store via lossy conversion
                    // (the binary will serve these from the filesystem anyway)
                    // For production, we'd use a separate binary asset pipeline
                }
            }
        }
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        let h1 = hash_content(b"hello world");
        let h2 = hash_content(b"hello world");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 8);
    }

    #[test]
    fn hash_changes_with_content() {
        let h1 = hash_content(b"hello");
        let h2 = hash_content(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_is_hex() {
        let h = hash_content(b"test");
        assert!(
            h.chars().all(|c| c.is_ascii_hexdigit()),
            "should be hex: {h}"
        );
    }

    #[test]
    fn insert_hash_with_extension() {
        assert_eq!(insert_hash("runtime.js", "a1b2c3d4"), "runtime.a1b2c3d4.js");
        assert_eq!(insert_hash("styles.css", "deadbeef"), "styles.deadbeef.css");
    }

    #[test]
    fn insert_hash_nested_name() {
        assert_eq!(insert_hash("page.home.js", "abc"), "page.home.abc.js");
    }

    #[test]
    fn insert_hash_no_extension() {
        assert_eq!(insert_hash("LICENSE", "abc12345"), "LICENSE.abc12345");
    }

    #[test]
    fn manifest_resolve() {
        let mut m = AssetManifest::new();
        m.insert(
            "_gale/runtime.js".into(),
            "_gale/runtime.a1b2c3d4.js".into(),
        );
        assert_eq!(
            m.resolve("_gale/runtime.js"),
            Some("_gale/runtime.a1b2c3d4.js")
        );
        assert_eq!(m.resolve("nonexistent"), None);
    }

    #[test]
    fn manifest_generates_valid_rust() {
        let mut m = AssetManifest::new();
        m.insert("_gale/runtime.js".into(), "_gale/runtime.abc.js".into());
        m.insert("_gale/styles.css".into(), "_gale/styles.def.css".into());
        let rs = m.generate_rust_module();
        assert!(rs.contains("HashMap"), "should use HashMap: {rs}");
        assert!(
            rs.contains("_gale/runtime.js"),
            "should contain logical path"
        );
        assert!(
            rs.contains("_gale/runtime.abc.js"),
            "should contain hashed path"
        );
        assert!(rs.contains("pub fn resolve"), "should have resolve fn");
        assert!(rs.contains("LazyLock"), "should use LazyLock");
    }

    #[test]
    fn manifest_to_json() {
        let mut m = AssetManifest::new();
        m.insert("a.js".into(), "a.abc.js".into());
        let json = m.to_json();
        assert!(json.contains("\"a.js\""));
        assert!(json.contains("\"a.abc.js\""));
    }

    #[test]
    fn hash_project_assets_renames_files() {
        let mut files = ProjectFiles::new();
        files.add_file("public/_gale/runtime.js", "console.log('hi');".into());
        files.add_file("public/_gale/styles.css", "body { }".into());
        files.add_file("public/favicon.ico", "icon".into()); // should NOT be hashed

        let manifest = hash_project_assets(&mut files);

        // Framework assets should be renamed
        assert_eq!(manifest.len(), 2);
        assert!(manifest.resolve("_gale/runtime.js").is_some());
        assert!(manifest.resolve("_gale/styles.css").is_some());

        // Hashed paths should contain the hash
        let hashed_js = manifest.resolve("_gale/runtime.js").unwrap();
        assert!(
            hashed_js.contains('.'),
            "should have hash in name: {hashed_js}"
        );
        assert!(
            hashed_js.ends_with(".js"),
            "should keep extension: {hashed_js}"
        );
        assert_ne!(
            hashed_js, "_gale/runtime.js",
            "should be different from original"
        );

        // User file should be untouched
        assert!(files.contains("public/favicon.ico"));
    }

    #[test]
    fn empty_manifest() {
        let m = AssetManifest::new();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
        let rs = m.generate_rust_module();
        assert!(rs.contains("pub fn resolve"));
    }
}
