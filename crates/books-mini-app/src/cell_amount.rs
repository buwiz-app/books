//! The one place a float exists: an Excel cell stores a number as a double.
//! [`number`] parses a report's two-place amount text into one. It does
//! no arithmetic, and refuses an amount of more than 15 significant digits
//! (the most that survives a decimal trip through a double and back) or text
//! that is not two-place. A value such as 0.01 is not exact in binary; what
//! is kept is that its digits read back the same.

use crate::service::Refusal;

pub fn number(t: &str) -> Result<f64, Refusal> {
    let body = t.strip_prefix('-').unwrap_or(t);
    let two_place = body
        .split_once('.')
        .is_some_and(|(w, c)| !w.is_empty() && c.len() == 2)
        && body.bytes().all(|b| b.is_ascii_digit() || b == b'.');
    if !two_place {
        return Err(Refusal::new("internal", "A report amount is not a number."));
    }
    if body.len() - 1 > 15 {
        return Err(Refusal::new(
            "too_large",
            "An amount in this report is too large for a spreadsheet cell.",
        ));
    }
    if body == "0.00" {
        return Ok(0.0);
    }
    t.parse()
        .map_err(|_| Refusal::new("internal", "A report amount is not a number."))
}
