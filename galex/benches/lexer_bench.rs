//! Criterion benchmarks for the GaleX lexer.
//!
//! Generates synthetic `.gx` source files and measures tokenization throughput.
//! Target: 10,000 lines in < 10ms.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use galex::{LexMode, Lexer};

/// Generate a synthetic GaleX source file with the specified number of lines.
/// Mixes declarations, functions, strings, template literals, numbers, and comments.
fn generate_gx_source(line_count: usize) -> String {
    let mut lines = Vec::with_capacity(line_count);
    for i in 0..line_count {
        let line = match i % 20 {
            0 => format!("let var_{} = {}", i, i * 7),
            1 => format!("mut counter_{}: int = {}", i, i),
            2 => format!("signal count_{} = {}", i, i % 100),
            3 => format!("derive doubled_{} = count_{} * 2", i, i - 1),
            4 => format!(
                "frozen CONFIG_{} = \"https://api.example.com/v{}/endpoint\"",
                i,
                i % 5
            ),
            5 => format!("fn process_{}(input: string, count: int) -> string {{", i),
            6 => format!(
                "  return `Result: ${{input}} count=${{count * {}}}`",
                i % 10
            ),
            7 => "  }".to_string(),
            8 => format!("guard User_{} {{", i),
            9 => format!("  name: string.min(2).max({})", 50 + i % 50),
            10 => format!("  email: string.email()"),
            11 => "}".to_string(),
            12 => format!("// Comment line {}: processing data for module", i),
            13 => format!("let hex_{} = 0xFF_{:02X} + 0b{:08b}", i, i % 256, i % 256),
            14 => format!("let float_{} = {}.{}", i, i % 1000, i % 100),
            15 => format!(
                "let expr_{} = (var_{} + {}) * {} - {} / {} % {}",
                i,
                i.saturating_sub(15),
                i,
                i % 10 + 1,
                i % 50,
                (i % 7) + 1,
                (i % 3) + 1
            ),
            16 => format!(
                "let optional_{} = data?.field?.nested ?? \"default_{}\"",
                i, i
            ),
            17 => format!("let piped_{} = input |> transform |> validate", i),
            18 => format!(
                "/* Block comment {} with some text about the implementation */",
                i
            ),
            19 => format!("let regex_{} = /^[a-z]+_{}_[0-9]{{2,4}}$/i", i, i),
            _ => unreachable!(),
        };
        lines.push(line);
    }
    lines.join("\n")
}

/// Generate a template-heavy GaleX source (simulating component bodies).
fn generate_template_source(line_count: usize) -> String {
    let mut lines = Vec::with_capacity(line_count);
    for i in 0..line_count {
        let line = match i % 10 {
            0 => format!("<div class=\"container-{}\">", i),
            1 => format!("  <span class=\"item\">{{}}</span>",),
            2 => format!("  <Button label=\"Click {}\" on:click={{handler}} />", i),
            3 => format!("  \"Text content for item {}\"", i),
            4 => format!("  <input bind:value type=\"text\" class:active={{true}} />"),
            5 => format!("  when condition_{} {{", i),
            6 => format!("    <p>\"Conditional content {}\"</p>", i),
            7 => "  }".to_string(),
            8 => format!("  {{compute_{}}}", i),
            9 => "</div>".to_string(),
            _ => unreachable!(),
        };
        lines.push(line);
    }
    lines.join("\n")
}

fn bench_lex_code(c: &mut Criterion) {
    let source_1k = generate_gx_source(1_000);
    let source_10k = generate_gx_source(10_000);

    let mut group = c.benchmark_group("lexer_code");

    group.throughput(Throughput::Bytes(source_1k.len() as u64));
    group.bench_function("lex_1k_lines", |b| {
        b.iter(|| {
            let result = galex::lex(black_box(&source_1k), 0);
            black_box(&result.tokens);
        });
    });

    group.throughput(Throughput::Bytes(source_10k.len() as u64));
    group.bench_function("lex_10k_lines", |b| {
        b.iter(|| {
            let result = galex::lex(black_box(&source_10k), 0);
            black_box(&result.tokens);
        });
    });

    group.finish();
}

fn bench_lex_template(c: &mut Criterion) {
    let template_1k = generate_template_source(1_000);

    let mut group = c.benchmark_group("lexer_template");
    group.throughput(Throughput::Bytes(template_1k.len() as u64));

    group.bench_function("lex_template_1k_lines", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(&template_1k), 0);
            lexer.push_mode(LexMode::Template);
            let tokens = lexer.tokenize_all();
            black_box(&tokens);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_lex_code, bench_lex_template);
criterion_main!(benches);
