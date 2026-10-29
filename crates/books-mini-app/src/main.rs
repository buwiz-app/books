//! `books-mini-app [mcp]`: serve MCP on stdio (what Rox spawns).
//! `books-mini-app manifest [--key-id <id>]`: print the unsigned manifest as
//! JCS text, under publisher key id `gc-2026-01` unless `--key-id` names a
//! test key id.

use std::io::Write;
use std::process::ExitCode;

use books_mini_app::jsonrpc::{Server, serve_stdio};
use books_mini_app::manifest::{self, Source};

const USAGE: &str = "usage: books-mini-app [mcp]\n       books-mini-app manifest [--key-id <id>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] | ["mcp"] => match serve_stdio(&mut Server::new(Source::from_env())) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("books-mini-app: stdio: {error}");
                ExitCode::FAILURE
            }
        },
        ["manifest"] => print_manifest(manifest::RELEASE_KEY_ID),
        ["manifest", "--key-id", key_id] if valid_key_id(key_id) => print_manifest(key_id),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
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
    let mut out = std::io::stdout().lock();
    match out
        .write_all(manifest::unsigned_jcs(key_id).as_bytes())
        .and_then(|()| out.flush())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
