use std::{env, fs, path::PathBuf};

const BOOKS_FILES: &[&str] = &[
    "../../apps/books/kit.tsx",
    "../../apps/books/suggested-categories.json",
    "../../apps/books/today.tsx",
    "../../apps/books/ledger_review.tsx",
    "../../apps/books/ledger_findings.tsx",
    "../../apps/books/ledger_amend.tsx",
    "../../apps/books/ledger_bulk.tsx",
    "../../apps/books/entry.tsx",
    "../../apps/books/entry_side.tsx",
    "../../apps/books/inbox.tsx",
    "../../apps/books/accounts.tsx",
    "../../apps/books/category_detail.tsx",
    "../../apps/books/unpaid.tsx",
    "../../apps/books/owed_print.tsx",
    "../../apps/books/owed_repeat.tsx",
    "../../apps/books/people.tsx",
    "../../apps/books/tags.tsx",
    "../../apps/books/reports.tsx",
    "../../apps/books/reconcile.tsx",
    "../../apps/books/reconcile_side.tsx",
    "../../apps/books/statement_read.tsx",
    "../../apps/books/statement_snapshot.tsx",
    "../../apps/books/reconcile_flags.tsx",
    "../../apps/books/settings.tsx",
    "../../apps/books/oldbooks.tsx",
    "../../apps/books/forms.tsx",
    "../../apps/books/payroll.tsx",
    "../../apps/books/certificates.tsx",
    "../../apps/books/activity.tsx",
    "../../apps/books/transactions_side.tsx",
    "../../apps/books/transactions.tsx",
    "../../apps/books/window.tsx",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Code,
    Template,
    Interpolation(usize),
    String(u8),
    BlockComment,
}

struct Scanner {
    modes: Vec<Mode>,
}

impl Scanner {
    fn new() -> Self {
        Self {
            modes: vec![Mode::Code],
        }
    }

    fn protects_line(&self) -> bool {
        self.modes.iter().any(|mode| {
            matches!(
                mode,
                Mode::Template | Mode::Interpolation(_) | Mode::String(_)
            )
        })
    }

    fn scan_line(&mut self, line: &str) {
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let mode = *self.modes.last().expect("scanner has a root mode");
            match mode {
                Mode::BlockComment => {
                    if bytes[i..].starts_with(b"*/") {
                        self.modes.pop();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                Mode::String(quote) => {
                    if bytes[i] == b'\\' {
                        i += 1;
                        if i < bytes.len() {
                            i += 1;
                        }
                    } else if bytes[i] == quote {
                        self.modes.pop();
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
                Mode::Template => {
                    if bytes[i] == b'\\' {
                        i += 1;
                        if i < bytes.len() {
                            i += 1;
                        }
                    } else if bytes[i..].starts_with(b"${") {
                        self.modes.push(Mode::Interpolation(1));
                        i += 2;
                    } else if bytes[i] == b'`' {
                        self.modes.pop();
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
                Mode::Code | Mode::Interpolation(_) => {
                    if bytes[i..].starts_with(b"//") {
                        break;
                    } else if bytes[i..].starts_with(b"/*") {
                        self.modes.push(Mode::BlockComment);
                        i += 2;
                    } else if matches!(bytes[i], b'\'' | b'"') {
                        self.modes.push(Mode::String(bytes[i]));
                        i += 1;
                    } else if bytes[i] == b'`' {
                        self.modes.push(Mode::Template);
                        i += 1;
                    } else if let Mode::Interpolation(depth) = mode {
                        if bytes[i] == b'{' {
                            *self.modes.last_mut().unwrap() =
                                Mode::Interpolation(depth.saturating_add(1));
                            i += 1;
                        } else if bytes[i] == b'}' {
                            if depth == 1 {
                                self.modes.pop();
                            } else {
                                *self.modes.last_mut().unwrap() = Mode::Interpolation(depth - 1);
                            }
                            i += 1;
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }
}

fn is_standalone_block_comment(trimmed: &str) -> bool {
    (trimmed.starts_with("/*") && trimmed.ends_with("*/") && trimmed.len() >= 4)
        || (trimmed.starts_with("{/*") && trimmed.ends_with("*/}") && trimmed.len() >= 6)
}

fn compact_tsx(source: &str, path: &str) -> String {
    let mut scanner = Scanner::new();
    let mut compacted = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        let protected = scanner.protects_line();
        let trimmed = line.trim();
        if protected {
            compacted.push_str(line);
        } else if trimmed.starts_with("//") || is_standalone_block_comment(trimmed) {
            if line.ends_with("\r\n") {
                compacted.push_str("\r\n");
            } else if line.ends_with('\n') {
                compacted.push('\n');
            }
        } else {
            compacted.push_str(line.trim_start_matches([' ', '\t']));
        }
        scanner.scan_line(line);
    }
    assert!(
        scanner.modes.len() == 1 && scanner.modes[0] == Mode::Code,
        "unclosed string, template, or comment in {path}"
    );
    compacted
}

fn compact_json_lines(source: &str) -> String {
    let mut compacted = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        compacted.push_str(line.trim_start_matches([' ', '\t']));
    }
    compacted
}

fn join_books(sources: &[String]) -> String {
    let mut joined = String::new();
    joined.push_str(&sources[0]);
    joined.push('\n');
    joined.push_str("const SUGGESTED_CATEGORY_CATALOG: Any[] = ");
    joined.push_str(&sources[1]);
    joined.push_str(";\n");
    let pages = &sources[2..];
    for (index, page) in pages.iter().enumerate() {
        joined.push_str(page);
        if index + 1 < pages.len() {
            joined.push('\n');
        }
    }
    joined
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    for path in BOOKS_FILES {
        println!("cargo:rerun-if-changed={path}");
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let sources = BOOKS_FILES
        .iter()
        .map(|path| fs::read_to_string(manifest_dir.join(path)))
        .collect::<Result<Vec<_>, _>>()?;
    let plain = join_books(&sources);
    let compacted_sources = BOOKS_FILES
        .iter()
        .zip(&sources)
        .map(|(path, source)| {
            if path.ends_with(".tsx") {
                compact_tsx(source, path)
            } else if path.ends_with(".json") {
                compact_json_lines(source)
            } else {
                source.to_owned()
            }
        })
        .collect::<Vec<_>>();
    let compacted = join_books(&compacted_sources);
    assert_eq!(
        plain.lines().count(),
        compacted.lines().count(),
        "Books source compaction changed the line count"
    );

    let bulk_index = BOOKS_FILES
        .iter()
        .position(|path| path.ends_with("/ledger_bulk.tsx"))
        .expect("Books file list includes the bulk panel");
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    fs::write(out_dir.join("books.tsx"), compacted)?;
    fs::write(
        out_dir.join("ledger_bulk.tsx"),
        &compacted_sources[bulk_index],
    )?;
    let plain_path = out_dir.join("books-plain.tsx");
    fs::write(&plain_path, &plain)?;
    println!(
        "cargo:rustc-env=BOOKS_PLAIN_SOURCE={}",
        plain_path.display()
    );
    println!(
        "cargo:rustc-env=BOOKS_PLAIN_LINE_COUNT={}",
        plain.lines().count()
    );
    println!("cargo:rustc-env=BOOKS_PLAIN_BYTES={}", plain.len());
    Ok(())
}
