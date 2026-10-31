//! M9: sign-in to buwiz-cloud and the tax-profile link (docs/01 5.7, P-C4,
//! P-C8). Optional (D19): Books works offline, and nothing here reaches the
//! network until the user signs in or lists tax profiles. A release build
//! signs in to `https://api.buwiz.com`; `BOOKS_CLOUD_URL` names another
//! cloud, or, set empty, turns the cloud off. A debug build has no cloud
//! unless `BOOKS_CLOUD_URL` names one.
//!
//! - Device sign-in as client `buwiz-books`, scope `books`: `POST
//!   /oauth/device/code`, then poll `POST /oauth/token`. The device code
//!   and the user code stay in memory.
//! - Refresh rotates, and reusing an old refresh token revokes the whole
//!   grant: Books saves each new refresh token in its encrypted store
//!   before it uses the access token that came with it.
//! - `GET /api/tax-profiles`: Books reads `id`, `tin_masked`,
//!   `registered_name` and `permissions` and ignores the rest; it refuses a
//!   response with a `tin` field or any TIN-shaped value.
//!
//! No token, code or TIN reaches a tool result, a log or the audit.

use books_core::privacy::tin_like;
use books_store::link;
use books_store::rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::service::{Answer, Refusal};

pub const CLIENT_ID: &str = "buwiz-books";
pub const SCOPE: &str = "books";
pub const URL_ENV: &str = "BOOKS_CLOUD_URL";
/// The cloud a release build uses when `BOOKS_CLOUD_URL` is unset.
pub const RELEASE_URL: &str = "https://api.buwiz.com";

/// How Books reaches the cloud: a form POST and a GET with a bearer token,
/// each answering a status and a JSON body.
pub trait Transport {
    fn post_form(&self, path: &str, form: &[(&str, &str)]) -> Result<(u16, Value), String>;
    fn get(&self, path: &str, bearer: &str) -> Result<(u16, Value), String>;
}

/// The cloud's base URL: `BOOKS_CLOUD_URL` when set, else [`RELEASE_URL`] in
/// a release build. Only `https://` counts, so an empty value means no cloud.
pub fn base_url(env: Option<&str>, release: bool) -> Option<String> {
    let base = match env {
        Some(url) => url.trim_end_matches('/'),
        None if release => RELEASE_URL,
        None => return None,
    };
    base.starts_with("https://").then(|| base.to_string())
}

/// The real transport: HTTPS to [`base_url`].
pub struct Http {
    base: String,
}

impl Http {
    pub fn from_env() -> Option<Self> {
        let env = std::env::var(URL_ENV).ok();
        let base = base_url(env.as_deref(), !cfg!(debug_assertions))?;
        Some(Self { base })
    }
}

fn body(r: Result<ureq::Response, ureq::Error>) -> Result<(u16, Value), String> {
    let resp = match r {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(e) => return Err(format!("the cloud could not be reached ({})", e.kind())),
    };
    let status = resp.status();
    let text = resp
        .into_string()
        .map_err(|_| "the cloud's answer could not be read".to_string())?;
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

impl Transport for Http {
    fn post_form(&self, path: &str, form: &[(&str, &str)]) -> Result<(u16, Value), String> {
        body(
            ureq::post(&format!("{}{path}", self.base))
                .timeout(std::time::Duration::from_secs(20))
                .send_form(form),
        )
    }

    fn get(&self, path: &str, bearer: &str) -> Result<(u16, Value), String> {
        body(
            ureq::get(&format!("{}{path}", self.base))
                .timeout(std::time::Duration::from_secs(20))
                .set("Authorization", &format!("Bearer {bearer}"))
                .call(),
        )
    }
}

/// A sign-in in progress, and the access token once signed in (memory only).
#[derive(Default)]
pub struct Session {
    device_code: Option<String>,
    access: Option<String>,
}

fn unreachable_cloud() -> Refusal {
    Refusal::new(
        "cloud_off",
        "Sign-in is not set up in this Books (no cloud address). Books works without it.",
    )
}

fn text_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

/// Start the device sign-in: the code the user types at the address.
pub fn start(t: &dyn Transport, s: &mut Session) -> Answer {
    let (status, v) = t
        .post_form(
            "/oauth/device/code",
            &[("client_id", CLIENT_ID), ("scope", SCOPE)],
        )
        .map_err(|e| Refusal::new("cloud", e))?;
    let (Some(device), Some(user), Some(uri)) = (
        text_of(&v, "device_code"),
        text_of(&v, "user_code"),
        text_of(&v, "verification_uri"),
    ) else {
        return Err(Refusal::new(
            "cloud",
            format!("The cloud did not start a sign-in (status {status})."),
        ));
    };
    s.device_code = Some(device.to_string());
    s.access = None;
    Ok(json!({
        "status": "waiting",
        "user_code": user,
        "verification_uri": uri,
        "verification_uri_complete": text_of(&v, "verification_uri_complete"),
        "interval": v.get("interval").and_then(Value::as_i64).unwrap_or(5).clamp(1, 60),
        "expires_in": v.get("expires_in").and_then(Value::as_i64).unwrap_or(600),
    }))
}

/// Keep the new refresh token, then the access token (P-C4).
fn keep(c: &Connection, s: &mut Session, v: &Value) -> Result<(), Refusal> {
    let (Some(access), Some(refresh)) = (text_of(v, "access_token"), text_of(v, "refresh_token"))
    else {
        return Err(Refusal::new(
            "cloud",
            "The cloud's tokens were incomplete. Sign in again.",
        ));
    };
    // the new refresh token is saved before the access token is used
    link::save_refresh_token(c, Some(refresh))?;
    s.access = Some(access.to_string());
    Ok(())
}

/// One poll of the device sign-in.
pub fn poll(t: &dyn Transport, c: &Connection, s: &mut Session) -> Answer {
    let device = s
        .device_code
        .clone()
        .ok_or_else(|| Refusal::new("not_started", "Start the sign-in first."))?;
    let (_, v) = t
        .post_form(
            "/oauth/token",
            &[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &device),
                ("client_id", CLIENT_ID),
            ],
        )
        .map_err(|e| Refusal::new("cloud", e))?;
    match text_of(&v, "error") {
        Some("authorization_pending") => Ok(json!({ "status": "waiting" })),
        Some("slow_down") => Ok(json!({ "status": "waiting", "slow_down": true })),
        Some("expired_token") => {
            s.device_code = None;
            Ok(json!({ "status": "expired" }))
        }
        Some("access_denied") => {
            s.device_code = None;
            Ok(json!({ "status": "denied" }))
        }
        Some(other) => Err(Refusal::new(
            "cloud",
            format!("The cloud refused the sign-in ({other})."),
        )),
        None => {
            keep(c, s, &v)?;
            s.device_code = None;
            Ok(json!({ "status": "signed_in" }))
        }
    }
}

/// A fresh access token from the saved refresh token (rotating it).
fn refresh(t: &dyn Transport, c: &Connection, s: &mut Session) -> Result<(), Refusal> {
    let token = link::refresh_token(c)?
        .ok_or_else(|| Refusal::new("signed_out", "Sign in to Buwiz first."))?;
    let (_, v) = t
        .post_form(
            "/oauth/token",
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", &token),
                ("client_id", CLIENT_ID),
            ],
        )
        .map_err(|e| Refusal::new("cloud", e))?;
    if text_of(&v, "error").is_some() {
        // a refused refresh ends the grant; sign in again
        link::save_refresh_token(c, None)?;
        s.access = None;
        return Err(Refusal::new(
            "signed_out",
            "The Buwiz sign-in ended. Sign in again.",
        ));
    }
    keep(c, s, &v)
}

/// Every string value in `v`, for the TIN check.
fn strings<'a>(v: &'a Value, out: &mut Vec<(&'a str, &'a str)>, key: &'a str) {
    match v {
        Value::String(s) => out.push((key, s)),
        Value::Array(a) => a.iter().for_each(|x| strings(x, out, key)),
        Value::Object(o) => o.iter().for_each(|(k, x)| strings(x, out, k)),
        _ => {}
    }
}

/// Read the cloud's tax-profile list: the four fields only, and refuse a
/// response that carries a TIN.
pub fn read_profiles(v: &Value) -> Result<Vec<Value>, Refusal> {
    let refuse = || {
        Refusal::new(
            "tin_refused",
            "The cloud sent a TIN. Books does not take TINs, so it did not read the list.",
        )
    };
    let mut all = Vec::new();
    strings(v, &mut all, "");
    if has_tin_key(v) || all.iter().any(|(k, s)| *k != "id" && tin_like(s)) {
        return Err(refuse());
    }
    let list = v
        .as_array()
        .or_else(|| v.get("tax_profiles").and_then(Value::as_array))
        .or_else(|| v.get("profiles").and_then(Value::as_array))
        .ok_or_else(|| Refusal::new("cloud", "The cloud's tax-profile list was not a list."))?;
    let mut out = Vec::new();
    for p in list {
        let Some(id) = text_of(p, "id").filter(|id| link::is_profile_id(id)) else {
            continue;
        };
        out.push(json!({
            "id": id.to_lowercase(),
            "tin_masked": text_of(p, "tin_masked"),
            "registered_name": text_of(p, "registered_name"),
            "permissions": p.get("permissions").cloned().unwrap_or(json!([])),
        }));
    }
    Ok(out)
}

fn has_tin_key(v: &Value) -> bool {
    match v {
        Value::Object(o) => o
            .iter()
            .any(|(k, x)| k.eq_ignore_ascii_case("tin") || has_tin_key(x)),
        Value::Array(a) => a.iter().any(has_tin_key),
        _ => false,
    }
}

/// The approved tax profiles the signed-in user owns.
pub fn profiles(t: &dyn Transport, c: &Connection, s: &mut Session) -> Result<Vec<Value>, Refusal> {
    if s.access.is_none() {
        refresh(t, c, s)?;
    }
    let mut tried = false;
    loop {
        let access = s.access.clone().unwrap_or_default();
        let (status, v) = t
            .get("/api/tax-profiles", &access)
            .map_err(|e| Refusal::new("cloud", e))?;
        if status == 401 && !tried {
            tried = true;
            refresh(t, c, s)?;
            continue;
        }
        if status != 200 {
            return Err(Refusal::new(
                "cloud",
                format!("The cloud did not list tax profiles (status {status})."),
            ));
        }
        return read_profiles(&v);
    }
}

/// `books_app_signin`: `start`, `poll` or `out`.
pub fn signin(
    t: Option<&dyn Transport>,
    c: &Connection,
    s: &mut Session,
    args: &Map<String, Value>,
) -> Answer {
    match args.get("step").and_then(Value::as_str).unwrap_or("start") {
        "out" => {
            link::save_refresh_token(c, None)?;
            *s = Session::default();
            Ok(json!({ "status": "signed_out" }))
        }
        "start" => start(t.ok_or_else(unreachable_cloud)?, s),
        "poll" => poll(t.ok_or_else(unreachable_cloud)?, c, s),
        _ => Err(Refusal::invalid("`step` is start, poll or out.")),
    }
}

/// Whether a refresh token is saved (signed in, as far as Books knows).
pub fn signed_in(c: &Connection) -> Result<bool, Refusal> {
    Ok(link::refresh_token(c)?.is_some())
}

/// Masked display for a linked profile, from the list.
pub fn masked(list: &[Value], id: &str) -> Option<String> {
    list.iter()
        .find(|p| p["id"] == id)
        .and_then(|p| text_of(p, "tin_masked"))
        .map(str::to_string)
}
