//! Buwiz Books as a Rox mini app (`dev.goldcoders.books`), protocol v1.
//!
//! - [`jsonrpc`]: the stdio MCP server Rox spawns;
//! - [`reserved`]: the reserved `rox.*` tools (provision, session
//!   certificates, ticket titles, uninstall);
//! - [`manifest`]: the manifest generator and the served (signed) manifest;
//! - [`jcs`]: RFC 8785 canonical JSON and I-JSON parsing, integers only;
//! - [`apps`]: the app sources served as `rox-app://<id>`;
//! - [`tools`]: Books' own `books_*` tools, with [`service`] (the store,
//!   tickets, idempotent audited writes), [`views`] (page data),
//!   [`writes`], [`records`] (people, tags, unpaid) and [`reports`].

pub mod apps;
pub mod b64;
pub mod demo;
pub mod forms;
pub mod jcs;
pub mod jsonrpc;
pub mod manifest;
pub mod oldbooks;
pub mod payroll;
pub mod reconcile;
pub mod records;
pub mod reports;
pub mod reserved;
pub mod service;
pub mod tools;
pub mod views;
pub mod writes;
