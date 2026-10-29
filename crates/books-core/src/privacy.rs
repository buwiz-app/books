//! Text a person types into Books (a book label, a payee, a memo, a
//! reference) must not carry a TIN (invariants 3 and 4). Books refuses it
//! with an explanation (open question Q-4, option a).
//!
//! A TIN is nine digits, optionally followed by a three- or five-digit
//! branch, written with or without dashes or spaces. Amounts are not text a
//! person types here, so the money caution of invariant 14 does not arise:
//! a number with a decimal point is not a TIN.

/// Whether `text` holds a run of nine or more digits, counting single
/// dashes or spaces between digit groups, outside a decimal number.
pub fn tin_like(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        let mut digits = 0;
        let mut j = i;
        while j < bytes.len() {
            if bytes[j].is_ascii_digit() {
                digits += 1;
                j += 1;
            } else if (bytes[j] == b'-' || bytes[j] == b' ')
                && j + 1 < bytes.len()
                && bytes[j + 1].is_ascii_digit()
                && j > start
            {
                j += 1;
            } else {
                break;
            }
        }
        let decimal =
            j < bytes.len() && bytes[j] == b'.' && bytes.get(j + 1).is_some_and(u8::is_ascii_digit);
        if digits >= 9 && !decimal {
            return true;
        }
        i = j.max(i + 1);
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
            "",
        ] {
            assert!(!tin_like(t), "{t}");
        }
    }
}
