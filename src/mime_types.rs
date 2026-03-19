/// Returns the MIME type for a given file extension.
///
/// Text types include `charset=utf-8`. Returns `application/octet-stream`
/// for unknown extensions.
///
/// Zero-allocation: uses a small stack buffer for case-insensitive
/// matching instead of `ext.to_ascii_lowercase()`.
#[inline]
pub fn from_extension(ext: &str) -> &'static str {
    // Fast path: lowercase directly into a stack buffer (extensions are short).
    let mut buf = [0u8; 16];
    let len = ext.len().min(16);
    buf[..len].copy_from_slice(&ext.as_bytes()[..len]);
    buf[..len].make_ascii_lowercase();
    let lower = std::str::from_utf8(&buf[..len]).unwrap_or("");

    match lower {
        // Web
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "jsonld" => "application/ld+json; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "svg" => "image/svg+xml; charset=utf-8",
        "wasm" => "application/wasm",
        "map" => "application/json; charset=utf-8",
        "webmanifest" => "application/manifest+json; charset=utf-8",

        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",

        // Fonts
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",

        // Audio
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "opus" => "audio/opus",
        "m4a" => "audio/mp4",

        // Video
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",

        // Documents
        "pdf" => "application/pdf",
        "txt" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "rtf" => "application/rtf",
        "md" => "text/markdown; charset=utf-8",

        // Archives
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "br" => "application/x-brotli",
        "zst" => "application/zstd",
        "tar" => "application/x-tar",
        "bz2" => "application/x-bzip2",
        "7z" => "application/x-7z-compressed",

        // Data
        "yaml" | "yml" => "text/yaml; charset=utf-8",
        "toml" => "text/toml; charset=utf-8",
        "rss" => "application/rss+xml; charset=utf-8",
        "atom" => "application/atom+xml; charset=utf-8",

        _ => "application/octet-stream",
    }
}
