// SPDX-License-Identifier: AGPL-3.0-only

//! Benchmarks for search / RAG path: text chunking (the most
//! computationally heavy local operation in the search pipeline).
//!
//! Covers critical paths identified in audit I-04:
//!   - chunk_text() — plain-text smart chunking
//!   - Chunk with Markdown heading awareness
//!   - Chunk with custom separator
//!   - Code-specific chunking

use axagent_search::text_chunker;
use criterion::{Criterion, criterion_group, criterion_main};

/// Generate a realistic Markdown document of approximately `n_lines`.
fn generate_markdown(n_lines: usize) -> String {
    let headings = [
        "# Introduction\n\n",
        "## Background\n\n",
        "### Prior Work\n\n",
        "## Methodology\n\n",
        "### Data Collection\n\n",
        "### Model Architecture\n\n",
        "## Results\n\n",
        "### Quantitative Evaluation\n\n",
        "### Qualitative Analysis\n\n",
        "## Conclusion\n\n",
    ];
    let body = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris \
                nisi ut aliquip ex ea commodo consequat.\n\n";

    let mut md = String::new();
    let mut i = 0;
    while md.lines().count() < n_lines {
        let h = headings[i % headings.len()];
        md.push_str(h);
        md.push_str(body);
        i += 1;
    }
    md
}

fn bench_chunk_text_short(c: &mut Criterion) {
    let short = "This is a short piece of text that fits within a single chunk.";
    c.bench_function("search_chunk_text_short", |b| {
        b.iter(|| {
            std::hint::black_box(text_chunker::chunk_text(
                std::hint::black_box(short),
                std::hint::black_box(2000),
                std::hint::black_box(200),
            ));
        })
    });
}

fn bench_chunk_text_medium(c: &mut Criterion) {
    // ~50 KB of plain text (mixed paragraphs)
    let medium = "Sed ut perspiciatis unde omnis iste natus error sit voluptatem \
                  accusantium doloremque laudantium, totam rem aperiam, eaque ipsa \
                  quae ab illo inventore veritatis et quasi architecto beatae vitae \
                  dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit \
                  aspernatur aut odit aut fugit.\n\n"
        .repeat(200);

    c.bench_function("search_chunk_text_medium", |b| {
        b.iter(|| {
            std::hint::black_box(text_chunker::chunk_text(
                std::hint::black_box(&medium),
                std::hint::black_box(2000),
                std::hint::black_box(200),
            ));
        })
    });
}

fn bench_chunk_markdown_large(c: &mut Criterion) {
    // ~200 KB Markdown document
    let md = generate_markdown(2000);

    c.bench_function("search_chunk_markdown_large", |b| {
        b.iter(|| {
            std::hint::black_box(text_chunker::chunk_text_with_separator_and_markdown(
                std::hint::black_box(&md),
                std::hint::black_box(2000),
                std::hint::black_box(200),
                std::hint::black_box(None),
                std::hint::black_box(true),
            ));
        })
    });
}

fn bench_chunk_code_style(c: &mut Criterion) {
    let code = "fn factorial(n: u64) -> u64 {\n    match n {\n        0 | 1 => 1,\n        _ => n * factorial(n - 1),\n    }\n}\n".repeat(300);

    c.bench_function("search_chunk_code", |b| {
        b.iter(|| {
            std::hint::black_box(text_chunker::chunk_text(
                std::hint::black_box(&code),
                std::hint::black_box(text_chunker::CODE_CHUNK_SIZE),
                std::hint::black_box(text_chunker::CODE_OVERLAP),
            ));
        })
    });
}

fn bench_chunk_custom_separator(c: &mut Criterion) {
    let sections = "---SECTION---\n".repeat(100) + &"Content for section.\n".repeat(2000);
    c.bench_function("search_chunk_custom_separator", |b| {
        b.iter(|| {
            std::hint::black_box(text_chunker::chunk_text_with_separator_and_markdown(
                std::hint::black_box(&sections),
                std::hint::black_box(2000),
                std::hint::black_box(200),
                std::hint::black_box(Some("---SECTION---")),
                std::hint::black_box(false),
            ));
        })
    });
}

criterion_group!(
    benches,
    bench_chunk_text_short,
    bench_chunk_text_medium,
    bench_chunk_markdown_large,
    bench_chunk_code_style,
    bench_chunk_custom_separator,
);
criterion_main!(benches);
