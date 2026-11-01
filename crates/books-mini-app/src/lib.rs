//! Buwiz Books as a Buwiz Bot mini app (`app.buwiz.books`), protocol 2.0.
//!
//! - [`jsonrpc`]: the stdio MCP server the Bot spawns;
//! - [`reserved`]: the reserved `buwiz.*` tools (provision, session
//!   certificates, ticket titles, uninstall);
//! - [`manifest`]: the manifest generator and the served (signed) manifest;
//! - [`jcs`]: RFC 8785 canonical JSON and I-JSON parsing, integers only;
//! - [`apps`]: the app sources served as `buwiz-app://<id>`;
//! - [`tools`]: Books' own `books_*` tools, with [`service`] (the store,
//!   tickets, idempotent audited writes), [`views`] (page data),
//!   [`writes`], [`records`] (people, tags, unpaid) and [`reports`].

pub mod apps;
pub mod b64;
pub mod cards;
pub mod checks;
pub mod cloud;
pub mod demo;
pub mod forms;
pub mod handoff;
pub mod html;
pub mod inbox;
pub mod jcs;
pub mod jsonrpc;
pub mod manifest;
pub mod match_assist;
pub mod oldbooks;
pub mod payroll;
pub mod reconcile;
pub mod reconcile_document;
pub mod records;
pub mod reports;
pub mod reserved;
pub mod service;
pub mod sheets;
pub mod statement_flags;
pub mod statement_read;
pub mod tools;
pub mod views;
pub mod writes;
