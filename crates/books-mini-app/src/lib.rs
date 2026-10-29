//! Buwiz Books as a Rox mini app (`dev.goldcoders.books`), protocol v1.
//!
//! - [`jsonrpc`]: the stdio MCP server Rox spawns;
//! - [`reserved`]: the reserved `rox.*` tools (provision, session
//!   certificates, ticket titles, uninstall);
//! - [`manifest`]: the manifest generator and the served (signed) manifest;
//! - [`jcs`]: RFC 8785 canonical JSON and I-JSON parsing, integers only;
//! - [`apps`]: the app sources served as `rox-app://<id>`;
//! - [`tools`]: Books' own `books_*` tools;
//! - [`tickets`]: random tickets in place of identifiers.

pub mod apps;
pub mod b64;
pub mod jcs;
pub mod jsonrpc;
pub mod manifest;
pub mod reserved;
pub mod tickets;
pub mod tools;
