//! The reserved tools of protocol v1 (spec 7.8): `rox.provision`,
//! `rox.session.open`, `rox.session.close`, `rox.ticket.resolve` and
//! `rox.uninstall`. Any other `rox.*` name is JSON-RPC -32601 (7.9).
//!
//! Rox generates one ed25519 key per install and sends its seed in
//! `rox.provision`, the first call of every spawn. The seed lives in
//! [`PluginKey`] and nowhere else: never written, logged, echoed or put in an
//! error message (P4), and `Debug` is not derived. Every other tool is
//! refused until a provision succeeds (P7).

use std::collections::HashMap;

use ed25519_dalek::{Signer as _, SigningKey};
use serde_json::{Map, Value, json};

use crate::jsonrpc::{fail, ok};
use crate::manifest::{self, Source};
use crate::tickets::Tickets;
use crate::{b64, jcs};

/// The reserved tools this plugin implements.
pub const RESERVED: [&str; 5] = [
    "rox.provision",
    "rox.session.open",
    "rox.session.close",
    "rox.ticket.resolve",
    "rox.uninstall",
];

/// The longest session a certificate may cover: 8 hours.
pub const CERT_MAX_MS: i64 = 8 * 3600 * 1000;

/// The provisioned plugin key.
pub struct PluginKey {
    signing: SigningKey,
}

impl PluginKey {
    /// `rox.provision`'s `key` and `pub`. The error never quotes either.
    pub fn from_provision(arguments: &Map<String, Value>) -> Result<Self, String> {
        let key = arguments
            .get("key")
            .and_then(Value::as_object)
            .ok_or("`key` must be {alg, seed}")?;
        if key.get("alg").and_then(Value::as_str) != Some("ed25519") {
            return Err("`key.alg` must be ed25519".into());
        }
        let seed: [u8; 32] = key
            .get("seed")
            .and_then(Value::as_str)
            .and_then(|seed| b64::decode_exact(seed, 32))
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or("`key.seed` must be 32 bytes of base64url")?;
        let signing = SigningKey::from_bytes(&seed);
        let public = b64::encode(signing.verifying_key().as_bytes());
        if arguments.get("pub").and_then(Value::as_str) != Some(public.as_str()) {
            return Err("the key and pub do not match".into());
        }
        Ok(Self { signing })
    }

    fn same_as(&self, other: &PluginKey) -> bool {
        self.signing.to_bytes() == other.signing.to_bytes()
    }

    /// base64url signature over the JCS bytes of `value`.
    pub fn sign(&self, value: &Value) -> Result<String, String> {
        let bytes = jcs::canonical(value).map_err(|error| error.to_string())?;
        Ok(b64::encode(&self.signing.sign(bytes.as_bytes()).to_bytes()))
    }
}

/// `rox.session.open`'s request, checked.
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
            "typ": "rox-session",
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
    /// Whether `rox.provision` has succeeded (P7).
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
        tickets: &mut Tickets,
        name: &str,
        arguments: &Map<String, Value>,
        now_ms: i64,
    ) -> Option<Result<Value, String>> {
        if !RESERVED.contains(&name) {
            return None;
        }
        if name != "rox.provision" && !self.provisioned() {
            return Some(Err("rox.provision must come first".into()));
        }
        Some(Ok(match name {
            "rox.provision" => self.provision(source, arguments),
            "rox.session.open" => self.session_open(source, arguments, now_ms),
            "rox.session.close" => {
                if let Some(open_id) = arguments.get("open_id").and_then(Value::as_str) {
                    self.sessions.remove(open_id);
                }
                ok(json!({ "ok": true }))
            }
            "rox.ticket.resolve" => {
                let ticket = arguments
                    .get("ticket")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match tickets.subject(ticket) {
                    Some(subject) => ok(json!({ "valid": true, "title": subject.title() })),
                    None => ok(json!({ "valid": false })),
                }
            }
            _ => {
                tickets.clear();
                self.sessions.clear();
                ok(json!({ "ok": true }))
            }
        }))
    }

    fn provision(&mut self, source: &Source, arguments: &Map<String, Value>) -> Value {
        if arguments.get("plugin").and_then(Value::as_str) != Some(manifest::PLUGIN_ID) {
            return fail("invalid_args", "this is dev.goldcoders.books");
        }
        let major = arguments
            .get("protocol")
            .and_then(Value::as_str)
            .and_then(|version| version.split('.').next());
        if major != Some("1") {
            return fail("invalid_args", "this plugin implements protocol 1");
        }
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
            return fail("invalid_args", "rox.provision must come first");
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
    use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};

    /// A provision request for a fixed test seed (never a real key).
    fn provision_args(seed: [u8; 32]) -> Map<String, Value> {
        let public = b64::encode(SigningKey::from_bytes(&seed).verifying_key().as_bytes());
        json!({
            "protocol": "1.0.0",
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
        let cert = json!({"v": 1, "typ": "rox-session", "not_after": 5});
        let sig: [u8; 64] = b64::decode_exact(&key.sign(&cert).unwrap(), 64)
            .unwrap()
            .try_into()
            .unwrap();
        let public = key.signing.verifying_key();
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
