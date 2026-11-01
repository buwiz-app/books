//! The reserved tools of protocol 2.0 (spec 7.8): `buwiz.provision`,
//! `buwiz.session.open`, `buwiz.session.close`, `buwiz.ticket.resolve` and
//! `buwiz.uninstall`. Any other `buwiz.*` name is JSON-RPC -32601 (7.9).
//!
//! The Bot generates one ed25519 key per install and sends its seed in
//! `buwiz.provision`, the first call of every spawn. The seed lives in
//! [`PluginKey`] and nowhere else: never written, logged, echoed or put in an
//! error message (P4), and `Debug` is not derived. Every other tool is
//! refused until a provision succeeds (P7).

use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::jsonrpc::{fail, ok};
use crate::manifest::{self, Source};
use buwiz_plugin::{CERT_MAX_MS, key::PluginKey};

use crate::b64;
use crate::service::Books;
pub use buwiz_plugin::RESERVED;

#[cfg(test)]
use ed25519_dalek::SigningKey;

/// `buwiz.session.open`'s request, checked.
struct OpenRequest {
    app: String,
    open_id: String,
    ks_pub: String,
    perms_sha256: String,
    not_after: i64,
}

impl OpenRequest {
    fn read(arguments: &Map<String, Value>, now_ms: i64) -> Result<Self, String> {
        let text = |key: &str| -> Result<String, String> {
            arguments
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("`{key}` must be a string"))
        };
        let app = text("app")?;
        let open_id = text("open_id")?;
        if b64::decode_exact(&open_id, 16).is_none() {
            return Err("`open_id` must be 128 bits of base64url".into());
        }
        let ks_pub = text("ks_pub")?;
        if b64::decode_exact(&ks_pub, 32).is_none() {
            return Err("`ks_pub` must be a 32-byte ed25519 key in base64url".into());
        }
        let perms_sha256 = text("perms_sha256")?;
        let not_after = arguments
            .get("not_after")
            .and_then(Value::as_i64)
            .ok_or("`not_after` must be an integer (unix ms)")?;
        if not_after <= now_ms || not_after > now_ms + CERT_MAX_MS {
            return Err("not_after must be within 8 h".into());
        }
        Ok(Self {
            app,
            open_id,
            ks_pub,
            perms_sha256,
            not_after,
        })
    }

    /// The certificate of spec 4.4.
    fn certificate(&self, plugin: &str) -> Value {
        json!({
            "v": 1,
            "typ": "buwiz-session",
            "plugin": plugin,
            "app": self.app,
            "open_id": self.open_id,
            "ks_pub": self.ks_pub,
            "perms_sha256": self.perms_sha256,
            "not_after": self.not_after,
        })
    }
}

/// What the reserved tools keep between requests.
#[derive(Default)]
pub struct Host {
    key: Option<PluginKey>,
    /// open_id -> app id, for opens this process certified.
    sessions: HashMap<String, String>,
}

impl Host {
    /// Whether `buwiz.provision` has succeeded (P7).
    pub fn provisioned(&self) -> bool {
        self.key.is_some()
    }

    /// The app of an open this process certified.
    pub fn session_app(&self, open_id: &str) -> Option<&str> {
        self.sessions.get(open_id).map(String::as_str)
    }

    /// A reserved tool. `None`: not one of ours (-32601). `Some(Err)`:
    /// refused before provisioning (-32600).
    pub fn call(
        &mut self,
        source: &Source,
        books: &mut Books,
        name: &str,
        arguments: &Map<String, Value>,
        now_ms: i64,
    ) -> Option<Result<Value, String>> {
        if !RESERVED.contains(&name) {
            return None;
        }
        if name != "buwiz.provision" && !self.provisioned() {
            return Some(Err("buwiz.provision must come first".into()));
        }
        Some(Ok(match name {
            "buwiz.provision" => self.provision(source, arguments),
            "buwiz.session.open" => self.session_open(source, arguments, now_ms),
            "buwiz.session.close" => {
                if let Some(open_id) = arguments.get("open_id").and_then(Value::as_str) {
                    self.sessions.remove(open_id);
                }
                ok(json!({ "ok": true }))
            }
            "buwiz.ticket.resolve" => {
                let ticket = arguments
                    .get("ticket")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match books.ticket_title(ticket) {
                    Some(title) => ok(json!({ "valid": true, "title": title })),
                    None => ok(json!({ "valid": false })),
                }
            }
            _ => {
                books.forget_tickets();
                self.sessions.clear();
                ok(json!({ "ok": true }))
            }
        }))
    }

    fn provision(&mut self, source: &Source, arguments: &Map<String, Value>) -> Value {
        if arguments.get("plugin").and_then(Value::as_str) != Some(manifest::PLUGIN_ID) {
            return fail("invalid_args", "this is app.buwiz.books");
        }
        // no check of `protocol` here: the Bot decides compatibility from
        // the manifest's `protocol` (2.0.0)
        let key = match PluginKey::from_provision(arguments) {
            Ok(key) => key,
            Err(message) => return fail("invalid_args", &message),
        };
        if let Some(held) = &self.key
            && !held.same_as(&key)
        {
            return fail("invalid_args", "already provisioned with another key");
        }
        let sha256 = match source.sha256() {
            Ok(sha256) => sha256,
            Err(message) => return fail("manifest_invalid", &message),
        };
        self.key = Some(key);
        ok(json!({ "ok": true, "manifest_sha256": sha256 }))
    }

    fn session_open(
        &mut self,
        source: &Source,
        arguments: &Map<String, Value>,
        now_ms: i64,
    ) -> Value {
        let Some(key) = &self.key else {
            return fail("invalid_args", "buwiz.provision must come first");
        };
        let request = match OpenRequest::read(arguments, now_ms) {
            Ok(request) => request,
            Err(message) => return fail("invalid_args", &message),
        };
        let app = match source.app(&request.app) {
            Ok(Some(app)) => app,
            Ok(None) => {
                let id: String = request.app.chars().take(60).collect();
                return fail("invalid_args", &format!("no app {id}"));
            }
            Err(message) => return fail("manifest_invalid", &message),
        };
        match manifest::permissions_digest(&app) {
            Ok(digest) if digest == request.perms_sha256 => {}
            Ok(_) => return fail("invalid_args", "perms_sha256 does not match the manifest"),
            Err(message) => return fail("manifest_invalid", &message),
        }
        let cert = request.certificate(manifest::PLUGIN_ID);
        let signature = match key.sign(&cert) {
            Ok(signature) => signature,
            Err(message) => return fail("invalid_args", &message),
        };
        self.sessions.insert(request.open_id, request.app);
        ok(json!({ "cert": cert, "cert_sig": signature }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jcs;
    use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};

    /// A provision request for a fixed test seed (never a real key).
    fn provision_args(seed: [u8; 32]) -> Map<String, Value> {
        let public = b64::encode(SigningKey::from_bytes(&seed).verifying_key().as_bytes());
        json!({
            "protocol": "2.0.0",
            "plugin": manifest::PLUGIN_ID,
            "key": {"alg": "ed25519", "seed": b64::encode(&seed)},
            "pub": public,
        })
        .as_object()
        .unwrap()
        .clone()
    }

    #[test]
    fn a_provisioned_key_signs_what_its_public_key_verifies() {
        let key = PluginKey::from_provision(&provision_args([7; 32])).unwrap();
        let cert = json!({"v": 1, "typ": "buwiz-session", "not_after": 5});
        let sig: [u8; 64] = b64::decode_exact(&key.sign(&cert).unwrap(), 64)
            .unwrap()
            .try_into()
            .unwrap();
        let public = VerifyingKey::from_bytes(
            &b64::decode_exact(&key.public(), 32)
                .unwrap()
                .try_into()
                .unwrap(),
        )
        .unwrap();
        let bytes = jcs::canonical(&cert).unwrap();
        assert!(
            public
                .verify(bytes.as_bytes(), &Signature::from_bytes(&sig))
                .is_ok()
        );
        let other =
            VerifyingKey::from_bytes(&SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes())
                .unwrap();
        assert!(
            other
                .verify(bytes.as_bytes(), &Signature::from_bytes(&sig))
                .is_err()
        );
    }

    #[test]
    fn a_mismatched_pub_or_bad_seed_is_refused_without_quoting_the_seed() {
        let seed = b64::encode(&[7; 32]);
        let mut args = provision_args([7; 32]);
        args.insert("pub".into(), json!(b64::encode(&[1; 32])));
        assert_eq!(
            PluginKey::from_provision(&args).err().unwrap(),
            "the key and pub do not match"
        );
        let mut args = provision_args([7; 32]);
        args["key"]["seed"] = json!("short");
        assert!(
            !PluginKey::from_provision(&args)
                .err()
                .unwrap()
                .contains(&seed)
        );
    }

    #[test]
    fn open_requests_hold_not_after_to_eight_hours() {
        let base = json!({
            "app": "home", "open_id": b64::encode(&[1; 16]), "ks_pub": b64::encode(&[2; 32]),
            "perms_sha256": "0".repeat(64), "not_after": 1_000 + CERT_MAX_MS,
        });
        let args = base.as_object().unwrap();
        assert!(OpenRequest::read(args, 1_000).is_ok());
        assert!(OpenRequest::read(args, 999).is_err());
    }
}
