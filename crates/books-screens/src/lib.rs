//! Books' native screens, `books.quiet` and `books.ledger`. The manifest
//! declares their ids and ABI; `books-mini-app` serves the TSX apps. Fit
//! follows layout, so fit counts can differ between the test stand-in and
//! the engine.

mod kit;
mod ledger;
mod quiet;

use buwiz_app::{ABI, ScreenDef};

pub fn screens() -> &'static [ScreenDef] {
    &[
        ScreenDef {
            id: "books.quiet",
            abi: ABI,
            build: quiet::build,
        },
        ScreenDef {
            id: "books.ledger",
            abi: ABI,
            build: ledger::build,
        },
    ]
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod ledger_tests;
