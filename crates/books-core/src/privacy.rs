//! Text a person types into Books (a book label, a payee, a memo, a
//! reference) must not carry a TIN (invariants 3 and 4). Books refuses it
//! with an explanation (open question Q-4, option a).
//!
//! A TIN is nine digits, optionally followed by a three- or five-digit
//! branch, written with or without dashes or spaces. Amounts are not text a
//! person types here, so the money caution of invariant 14 does not arise:
//! a number with a decimal point is not a TIN.

/// Whether `text` holds a TIN shape: nine or more digits in a row, or
/// groups of 3-3-3 (and an optional branch of 3 to 5) joined by single
/// dashes or spaces, outside a decimal number. A date or a reference such
/// as `4021-2026-10` is not one.
pub fn tin_like(text: &str) -> bool {
    let b = text.as_bytes();
    let digit = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    let mut i = 0;
    while i < b.len() {
        if !digit(i) || (i > 0 && digit(i - 1)) {
            i += 1;
            continue;
        }
        // a run of digits starting here
        let mut j = i;
        while digit(j) {
            j += 1;
        }
        let decimal = b.get(j) == Some(&b'.') && digit(j + 1);
        if j - i >= 9 && !decimal {
            return true;
        }
        // 3-3-3 groups
        if j - i == 3 {
            let mut k = j;
            let mut groups = 1;
            while groups < 3 && matches!(b.get(k), Some(b'-' | b' ')) && digit(k + 1) {
                let mut m = k + 1;
                while digit(m) {
                    m += 1;
                }
                if m - (k + 1) != 3 {
                    break;
                }
                groups += 1;
                k = m;
            }
            if groups == 3 && !(b.get(k) == Some(&b'.') && digit(k + 1)) {
                return true;
            }
        }
        i = j;
    }
    false
}

/// A refusal message when `text` looks like it holds a TIN.
pub fn refuse_tin(field: &str, text: &str) -> Result<(), String> {
    if tin_like(text) {
        Err(format!(
            "The {field} looks like it holds a TIN. Books never stores a TIN; leave it out."
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TIN shapes are built at run time, so no source file carries one.
    fn shapes() -> Vec<String> {
        let (a, b, c) = ("123", "456", "789");
        vec![
            format!("{a}{b}{c}"),
            format!("{a}-{b}-{c}"),
            format!("{a} {b} {c} 000"),
            format!("TIN {a}-{b}-{c}-00000"),
            format!("ref{a}{b}{c}x"),
        ]
    }

    #[test]
    fn tin_shapes_are_caught() {
        for t in shapes() {
            assert!(tin_like(&t), "{t}");
        }
        for t in [
            "Invoice 0142",
            "Bayani Foods",
            "12345678",
            "300000000.00",
            "Oct 1-15",
            "Card 7713",
            "bank-4021-2026-10.csv",
            "Oct 1-15",
            "",
        ] {
            assert!(!tin_like(t), "{t}");
        }
    }
}
