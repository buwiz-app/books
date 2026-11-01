//! base64url without padding (RFC 4648 §5), as v1 writes keys, seeds,
//! open ids and signatures. The implementation is the SDK's.

pub use buwiz_plugin::b64::{decode, decode_exact, encode};
