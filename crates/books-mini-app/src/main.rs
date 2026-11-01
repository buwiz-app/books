//! `books-mini-app [mcp]`: serve MCP on stdio (what the Bot spawns).
//! `books-mini-app manifest [--key-id <id>]`: print the unsigned manifest as
//! JCS text, under publisher key id `gc-2026-01` unless `--key-id` names a
//! test key id.
//! `books-mini-app app-source <id>`: print a TSX app's served source, for the
//! signing step (`sign-manifest.mjs --app <id>=<file>`).
//! `books-mini-app seed <demo.json>`: replay a synthetic demo book into the
//! store `BOOKS_DATA_DIR` names (the test the Bot and the golden tests).
//! `books-mini-app package <plugin.json> <out> <platform> [--published <plugin.json>]`:
//! fill a release's plugin.json (protocol 1.0 section 7) from the signed
//! manifest and the binary in `<out>`, for `buwiz-broker sign-manifest
//! --block buwiz` (the SDK's `package`; `tools/release/publish.sh` runs it).

use std::io::Write;
use std::process::ExitCode;

use books_mini_app::jsonrpc::{Server, serve_stdio};
use books_mini_app::manifest::{self, Source};
use books_mini_app::service::Books;

const USAGE: &str = "usage: books-mini-app [mcp]\n       books-mini-app manifest [--key-id <id>]\n       books-mini-app app-source <app id>\n       books-mini-app seed <demo.json>\n       books-mini-app package <plugin.json> <out> <platform> [--published <plugin.json>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["mcp"] => {
            match serve_stdio(&mut Server::new(Source::from_env(), Books::from_env())) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("books-mini-app: stdio: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        ["manifest"] => print_manifest(manifest::RELEASE_KEY_ID),
        ["manifest", "--key-id", key_id] if valid_key_id(key_id) => print_manifest(key_id),
        ["seed", file] => seed(file),
        ["package", template, out, platform] => package(template, out, platform, None),
        ["package", template, out, platform, "--published", published] => {
            package(template, out, platform, Some(published))
        }
        ["app-source", id] => match books_mini_app::apps::find(id) {
            Some(app) => match app.source {
                books_mini_app::apps::AppSource::Tsx(source) => print_text(source),
                books_mini_app::apps::AppSource::Native { .. } => {
                    eprintln!("books-mini-app: app {id} is native and has no TSX source");
                    ExitCode::from(2)
                }
            },
            None => {
                eprintln!("books-mini-app: no app {id}");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// The release's plugin.json to sign: the binary is `books-mini-app`, so its
/// asset is `books-mini-app-<platform>` and the signed manifest
/// `app.buwiz.books.manifest.json`.
fn package(template: &str, out: &str, platform: &str, published: Option<&str>) -> ExitCode {
    use std::path::Path;
    match buwiz_plugin::package::run(
        Path::new(template),
        Path::new(out),
        manifest::PLUGIN_ID,
        "books-mini-app",
        platform,
        published.map(Path::new),
    ) {
        Ok(()) => {
            eprintln!(
                "books-mini-app: wrote {out}/{}",
                buwiz_plugin::package::UNSIGNED
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("books-mini-app: package: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Replay a synthetic demo book into the store `BOOKS_DATA_DIR` names.
fn seed(file: &str) -> ExitCode {
    let demo = match std::fs::read_to_string(file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
    {
        Some(d) => d,
        None => {
            eprintln!("books-mini-app: {file} is not a demo file");
            return ExitCode::from(2);
        }
    };
    match books_mini_app::demo::seed(&mut Books::from_env(), &demo) {
        Ok(n) => {
            println!("seeded {n} steps");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("books-mini-app: seed: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `^[a-z0-9][a-z0-9-]{0,63}$` (spec 5.4).
fn valid_key_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0] != b'-'
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

fn print_manifest(key_id: &str) -> ExitCode {
    print_text(&manifest::unsigned_jcs(key_id))
}

/// Exactly these bytes on stdout (the signer fingerprints them).
fn print_text(text: &str) -> ExitCode {
    let mut out = std::io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
