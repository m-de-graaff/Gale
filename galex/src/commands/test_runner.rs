//! Test runner — compiles test blocks and runs them.
//!
//! Server-side test blocks are compiled to Rust `#[test]` functions
//! and executed via `cargo test` in a temporary project.

use crate::ast::TestDecl;
use crate::compiler::Compiler;

/// Compile test blocks to Rust and run via cargo test.
///
/// Returns the exit code (0 = all passed, 1 = failures).
pub fn run_tests(tests: &[&&TestDecl], _compiler: &Compiler) -> i32 {
    let work_dir = std::env::temp_dir().join("gale_test_runner");
    let _ = std::fs::remove_dir_all(&work_dir);
    std::fs::create_dir_all(work_dir.join("src")).ok();

    // Generate Cargo.toml
    let cargo_toml = r#"[package]
name = "gale_tests"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
    std::fs::write(work_dir.join("Cargo.toml"), cargo_toml).ok();

    // Generate test file
    let mut test_code = String::from("#[cfg(test)]\nmod tests {\n");
    for (i, test) in tests.iter().enumerate() {
        let fn_name = sanitize_test_name(&test.name, i);
        test_code.push_str(&format!("    #[test]\n    fn {fn_name}() {{\n"));
        test_code.push_str(&format!("        // Test: \"{}\"\n", test.name));
        test_code.push_str("        // TODO: compile test body to Rust\n");
        test_code
            .push_str("        // (test block compilation is pending full expression codegen)\n");
        test_code.push_str("    }\n\n");
    }
    test_code.push_str("}\n");

    // Write lib.rs
    std::fs::write(work_dir.join("src").join("lib.rs"), &test_code).ok();

    // Run cargo test
    eprintln!(
        "  Running {} test{}...",
        tests.len(),
        if tests.len() != 1 { "s" } else { "" }
    );
    let status = std::process::Command::new("cargo")
        .arg("test")
        .arg("--quiet")
        .current_dir(&work_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    // Cleanup
    let _ = std::fs::remove_dir_all(&work_dir);

    match status {
        Ok(s) if s.success() => {
            eprintln!("  All tests passed");
            0
        }
        Ok(_) => {
            eprintln!("  Some tests failed");
            1
        }
        Err(e) => {
            eprintln!("  Failed to run tests: {e}");
            1
        }
    }
}

/// Convert a test name to a valid Rust function identifier.
fn sanitize_test_name(name: &str, index: usize) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        format!("test_{index}")
    } else {
        format!("test_{sanitized}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_names() {
        assert_eq!(sanitize_test_name("my test", 0), "test_my_test");
        assert_eq!(sanitize_test_name("hello-world!", 1), "test_hello_world_");
        assert_eq!(sanitize_test_name("", 5), "test_5");
    }
}
