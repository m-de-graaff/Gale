//! Package validation (GX2000-GX2009).
//!
//! Factory functions for package/dependency diagnostics. Used by the
//! registry client and dependency resolver to report issues as
//! structured [`Diagnostic`] values.

use crate::errors::{codes, Diagnostic};
use crate::span::Span;

/// GX2000: Package not found in registry.
pub fn package_not_found(name: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX2000,
        format!("Package `{}` not found in registry", name),
        Span::dummy(),
    )
}

/// GX2001: No compatible version for the requested package.
pub fn no_compatible_version(name: &str, requested: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX2001,
        format!("Package `{}@{}` has no compatible version", name, requested),
        Span::dummy(),
    )
}

/// GX2002: Package requires a newer Gale version.
pub fn requires_newer_gale(name: &str, required: &str, current: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX2002,
        format!(
            "Package `{}` requires Gale >= {}, current is {}",
            name, required, current
        ),
        Span::dummy(),
    )
}

/// GX2003: Package checksum mismatch — possible tampering.
pub fn checksum_mismatch(name: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX2003,
        format!("Package `{}` checksum mismatch — possible tampering", name),
        Span::dummy(),
    )
}

/// GX2004: Lockfile is out of date.
pub fn lockfile_outdated() -> Diagnostic {
    Diagnostic::new(&codes::GX2004, Span::dummy())
}

/// GX2005: Package has a known vulnerability (warning).
pub fn known_vulnerability(name: &str, cve: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX2005,
        format!("Package `{}` has known vulnerability: {}", name, cve),
        Span::dummy(),
    )
}

/// GX2006: Package is deprecated (warning).
pub fn deprecated_package(name: &str, replacement: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX2006,
        format!(
            "Package `{}` is deprecated — use `{}` instead",
            name, replacement
        ),
        Span::dummy(),
    )
}

/// GX2007: Circular package dependency.
pub fn circular_dependency(a: &str, b: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX2007,
        format!("Circular package dependency: `{}` <-> `{}`", a, b),
        Span::dummy(),
    )
}

/// GX2008: Package contains invalid .gx files.
pub fn invalid_package_files(name: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX2008,
        format!("Package `{}` contains invalid `.gx` files", name),
        Span::dummy(),
    )
}

/// GX2009: Unused package (warning).
pub fn unused_package(name: &str) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX2009,
        format!("Unused package `{}`", name),
        Span::dummy(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_not_found_message() {
        let diag = package_not_found("db/postgres");
        assert_eq!(diag.code.code, 2000);
        assert!(diag.message.contains("db/postgres"));
        assert!(diag.is_error());
    }

    #[test]
    fn no_compatible_version_message() {
        let diag = no_compatible_version("ui/tabs", "^2.0");
        assert_eq!(diag.code.code, 2001);
        assert!(diag.message.contains("ui/tabs@^2.0"));
    }

    #[test]
    fn requires_newer_gale_message() {
        let diag = requires_newer_gale("auth/oauth", "0.5.0", "0.4.2");
        assert_eq!(diag.code.code, 2002);
        assert!(diag.message.contains("0.5.0"));
        assert!(diag.message.contains("0.4.2"));
    }

    #[test]
    fn checksum_mismatch_message() {
        let diag = checksum_mismatch("util/crypto");
        assert_eq!(diag.code.code, 2003);
        assert!(diag.message.contains("tampering"));
    }

    #[test]
    fn lockfile_outdated_message() {
        let diag = lockfile_outdated();
        assert_eq!(diag.code.code, 2004);
    }

    #[test]
    fn known_vulnerability_is_warning() {
        let diag = known_vulnerability("old-lib", "CVE-2024-1234");
        assert_eq!(diag.code.code, 2005);
        assert!(diag.is_warning());
        assert!(diag.message.contains("CVE-2024-1234"));
    }

    #[test]
    fn deprecated_package_is_warning() {
        let diag = deprecated_package("old-auth", "new-auth");
        assert_eq!(diag.code.code, 2006);
        assert!(diag.is_warning());
        assert!(diag.message.contains("new-auth"));
    }

    #[test]
    fn circular_dependency_message() {
        let diag = circular_dependency("a", "b");
        assert_eq!(diag.code.code, 2007);
        assert!(diag.message.contains("a"));
        assert!(diag.message.contains("b"));
    }

    #[test]
    fn unused_package_is_warning() {
        let diag = unused_package("unused-lib");
        assert_eq!(diag.code.code, 2009);
        assert!(diag.is_warning());
    }
}
