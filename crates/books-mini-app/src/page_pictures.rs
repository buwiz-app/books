//! A PDF's pages as pictures. The Books window draws PNG and JPEG only, so
//! when a PDF statement is kept its pages are drawn once, here, and stored
//! beside the document (`evidence_page`). This is drawing, not reading: no
//! text, line or amount is taken from the page. It runs inside the write
//! that keeps the document or accepts its statement read, never in a read.
//!
//! The drawing is hayro's, in Rust alone, so it is the same on Linux, macOS
//! and Windows and Books stays one binary. A page hayro cannot draw has no
//! picture, and the window says so and offers "See the page".

use std::panic::{AssertUnwindSafe, catch_unwind};

use books_store::evidence;
use books_store::rusqlite::Connection;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings, render};

use crate::reconcile_document::MAX_PAGE_IMAGE;
use crate::service::Refusal;

/// The pages of one PDF that get a picture: the first hundred.
pub const MAX_PAGES: usize = 100;

/// Each page of `pdf` as a PNG, in page order; `None` for a page that could
/// not be drawn or whose picture is over the window's bound. Empty when the
/// file cannot be opened (a password, or not a PDF).
pub fn pdf_pages(pdf: &[u8]) -> Vec<Option<Vec<u8>>> {
    // a malformed file must not take the write down with it
    catch_unwind(AssertUnwindSafe(|| draw(pdf))).unwrap_or_default()
}

fn draw(bytes: &[u8]) -> Vec<Option<Vec<u8>>> {
    let Ok(pdf) = Pdf::new(bytes.to_vec()) else {
        return Vec::new();
    };
    let cache = RenderCache::new();
    let settings = PixmapSettings {
        // pixels for each PDF point: a letter page is 918 by 1188. Page
        // geometry, not money: the one place Books holds a fraction.
        x_scale: 1.5,
        y_scale: 1.5,
        bg_color: WHITE,
    };
    pdf.pages()
        .iter()
        .take(MAX_PAGES)
        .map(|page| {
            let (w, h) = page.render_dimensions();
            // in points: a poster is not drawn
            if !(w >= 1.0 && h >= 1.0 && w <= 2000.0 && h <= 2000.0) {
                return None;
            }
            catch_unwind(AssertUnwindSafe(|| {
                render(
                    page,
                    &cache,
                    &InterpreterSettings::default(),
                    &RenderSettings::default(),
                    &settings,
                )
                .into_png()
                .ok()
            }))
            .ok()
            .flatten()
            .filter(|png| png.len() <= MAX_PAGE_IMAGE)
        })
        .collect()
}

/// Draw and keep the pages of a PDF document that has none kept yet. Any
/// other document, and a PDF already drawn, is left as it is.
pub fn keep(conn: &Connection, evidence_id: i64) -> Result<(), Refusal> {
    let Some(meta) = evidence::get(conn, evidence_id)? else {
        return Ok(());
    };
    if meta.mime != "application/pdf" || !evidence::pages(conn, evidence_id)?.is_empty() {
        return Ok(());
    }
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM evidence WHERE id = ?1",
            [evidence_id],
            |r| r.get(0),
        )
        .map_err(books_store::StoreError::from)?;
    let pages: Vec<(i64, Vec<u8>)> = pdf_pages(&bytes)
        .into_iter()
        .enumerate()
        .filter_map(|(i, png)| Some((i64::try_from(i + 1).ok()?, png?)))
        .collect();
    evidence::keep_pages(conn, evidence_id, &pages)?;
    Ok(())
}
