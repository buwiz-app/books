//! JSON Canonicalization Scheme (RFC 8785) and I-JSON parsing for the
//! objects protocol 1.0 signs and hashes: the manifest, session
//! certificates and permissions. The implementation is the SDK's, byte
//! for byte the same code that was here.

pub use buwiz_plugin::jcs::{
    JcsError, MAX_SAFE, canonical, canonical_text, parse, sha256_hex, sha256_jcs,
};
