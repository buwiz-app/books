mod kit;
mod quiet;

use buwiz_app::{ABI, ScreenDef};

pub fn screens() -> &'static [ScreenDef] {
    &[ScreenDef {
        id: "books.quiet",
        abi: ABI,
        build: quiet::build,
    }]
}

#[cfg(test)]
mod tests;
