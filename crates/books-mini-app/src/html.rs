//! Static HTML for `buwiz.preview` documents: escaping, the page shell and
//! standard base64 (RFC 4648 §4, padded) for `data:` URIs. The preview CSP
//! allows inline style and `data:` images and objects, never script.

/// `text` with `& < > " '` replaced by entities.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// A complete static page. `title` is escaped; `style` and `body` are
/// markup the caller built from escaped text.
pub fn page(title: &str, style: &str, body: &str) -> String {
    format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title><style>{style}</style></head><body>{body}</body></html>",
        escape(title)
    )
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The length of [`base64`] of `len` bytes.
pub fn base64_len(len: usize) -> usize {
    len.div_ceil(3).saturating_mul(4)
}

/// Standard padded base64 of `bytes`.
pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(base64_len(bytes.len()));
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | (u32::from(b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_all_five_characters() {
        assert_eq!(escape("&"), "&amp;");
        assert_eq!(escape("<"), "&lt;");
        assert_eq!(escape(">"), "&gt;");
        assert_eq!(escape("\""), "&quot;");
        assert_eq!(escape("'"), "&#39;");
        assert_eq!(
            escape("<b>Tom & Jerry's \"x\"</b>"),
            "&lt;b&gt;Tom &amp; Jerry&#39;s &quot;x&quot;&lt;/b&gt;"
        );
        assert_eq!(escape("Coffee shop ₱"), "Coffee shop ₱");
    }

    #[test]
    fn base64_matches_rfc_4648_vectors() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), expected);
            assert_eq!(base64_len(input.len()), expected.len());
        }
        assert_eq!(base64(&[0xfb, 0xff, 0xfe]), "+//+");
    }

    #[test]
    fn page_escapes_its_title() {
        let p = page("A <b>", "p{}", "<p>x</p>");
        assert!(p.starts_with("<!DOCTYPE html>"));
        assert!(p.contains("<title>A &lt;b&gt;</title>"));
        assert!(!p.contains("<script"));
    }
}
