//! JSON-RPC over the real binary's stdio: `initialize`, `tools/list`, the
//! reserved tools in their order, resources, and an app call through a
//! certified open. Fixed test seeds only; never a real key.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use books_mini_app::{b64, jcs, manifest};
use ed25519_dalek::{Signature, SigningKey, Verifier as _};
use serde_json::{Value, json};

struct Plugin {
    _data: books_store::TempDir,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: i64,
}

impl Plugin {
    fn spawn() -> Self {
        let data = books_store::TempDir::new("stdio");
        let mut child = Command::new(env!("CARGO_BIN_EXE_books-mini-app"))
            .env_remove(manifest::MANIFEST_ENV)
            .env(books_store::DATA_DIR_ENV, data.0.join("store"))
            .env_remove(books_store::KEY_FILE_ENV)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn books-mini-app");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Self {
            _data: data,
            child,
            stdin,
            stdout,
            next: 1,
        }
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        let reply: Value = serde_json::from_str(&line).expect("one JSON line");
        assert_eq!(reply["jsonrpc"], "2.0");
        assert_eq!(reply["id"], id);
        reply
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
    }

    fn call_as(&mut self, name: &str, arguments: Value, caller: Value) -> Value {
        self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments, "_meta": { "buwiz/caller": caller } }),
        )
    }

    /// Close stdin; the process exits and its stderr is returned.
    fn finish(self) -> String {
        drop(self.stdin);
        let output = self.child.wait_with_output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stderr).unwrap()
    }
}

fn now_ms() -> i64 {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    i64::try_from(elapsed.as_millis()).unwrap()
}

#[test]
fn initialize_list_provision_open_and_call() {
    let mut plugin = Plugin::spawn();

    // initialize, then the initialized notification (no answer)
    let init = plugin.request("initialize", json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }));
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "books-mini-app");
    assert!(init["result"]["capabilities"]["tools"].is_object());
    assert!(init["result"]["capabilities"]["resources"].is_object());
    plugin.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));

    // tools/list: the five reserved tools, then books_* only
    let list = plugin.request("tools/list", json!({}));
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names[..5],
        [
            "buwiz.provision",
            "buwiz.session.open",
            "buwiz.session.close",
            "buwiz.ticket.resolve",
            "buwiz.uninstall",
        ]
    );
    assert_eq!(names[5..], books_mini_app::tools::names()[..]);

    // resources are readable before provision; tools are not (P7)
    let read = plugin.request("resources/read", json!({ "uri": "buwiz://manifest" }));
    let served = read["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(served, manifest::unsigned_jcs(manifest::RELEASE_KEY_ID));
    let source = plugin.request("resources/read", json!({ "uri": "buwiz-app://home" }));
    assert_eq!(
        source["result"]["contents"][0]["mimeType"],
        "text/x-nsh-tsx"
    );
    let resources = plugin.request("resources/list", json!({}));
    assert!(
        !resources["result"]["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| resource["uri"] == "buwiz-app://quiet")
    );
    let native = plugin.request("resources/read", json!({ "uri": "buwiz-app://quiet" }));
    assert_eq!(native["error"]["code"], -32602);
    assert_eq!(
        native["error"]["message"],
        "native app quiet has no TSX source"
    );
    assert_eq!(
        plugin.request("resources/read", json!({ "uri": "buwiz-app://nope" }))["error"]["code"],
        -32602
    );
    assert_eq!(
        plugin.call("books_status", json!({}))["error"]["code"],
        -32600
    );
    assert_eq!(
        plugin.call("buwiz.session.open", json!({}))["error"]["code"],
        -32600
    );

    // buwiz.provision, twice with the same key; another key is refused
    let seed = [7u8; 32];
    let signing = SigningKey::from_bytes(&seed);
    let public = b64::encode(signing.verifying_key().as_bytes());
    let provision = json!({
        "protocol": "1.0.0", "plugin": "app.buwiz.books",
        "key": { "alg": "ed25519", "seed": b64::encode(&seed) }, "pub": public,
    });
    let expected_sha = jcs::sha256_jcs(&jcs::parse(&served).unwrap()).unwrap();
    for _ in 0..2 {
        let answer = plugin.call("buwiz.provision", provision.clone());
        assert_eq!(answer["result"]["isError"], false, "{answer}");
        assert_eq!(
            answer["result"]["structuredContent"],
            json!({ "ok": true, "manifest_sha256": expected_sha })
        );
    }
    let other = [9u8; 32];
    let refused = plugin.call(
        "buwiz.provision",
        json!({ "protocol": "1.0.0", "plugin": "app.buwiz.books",
                "key": { "alg": "ed25519", "seed": b64::encode(&other) },
                "pub": b64::encode(SigningKey::from_bytes(&other).verifying_key().as_bytes()) }),
    );
    assert_eq!(
        refused["result"]["structuredContent"]["code"],
        "invalid_args"
    );

    // an unknown buwiz.* is -32601 (10.1); so are the account tools, since
    // Books declares no accounts, and a push is never taken for an uninstall
    assert_eq!(
        plugin.call("buwiz.sdk.probe", json!({}))["error"]["code"],
        -32601
    );
    for name in ["buwiz.account.set", "buwiz.account.clear"] {
        let push =
            json!({ "account": "google", "label": "default", "access_token": "synthetic-token" });
        let answer = plugin.call(name, push);
        assert_eq!(answer["error"]["code"], -32601, "{name}: {answer}");
        assert!(
            !answer.to_string().contains("synthetic-token"),
            "{name}: {answer}"
        );
    }

    // buwiz.session.open: the certificate is the request, signed over JCS
    let home = manifest::unsigned(manifest::RELEASE_KEY_ID)["apps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "home")
        .unwrap()
        .clone();
    let perms = manifest::permissions_digest(&home).unwrap();
    let open_id = b64::encode(&[1u8; 16]);
    let not_after = now_ms() + 3_600_000;
    let open = |app: &str, perms: &str, not_after: i64| json!({ "app": app, "open_id": open_id, "ks_pub": b64::encode(&[2u8; 32]), "perms_sha256": perms, "not_after": not_after });
    for bad in [
        open("nope", &perms, not_after),
        open("home", &"0".repeat(64), not_after),
        open("home", &perms, now_ms() + 9 * 3_600_000),
    ] {
        let answer = plugin.call("buwiz.session.open", bad);
        assert_eq!(
            answer["result"]["structuredContent"]["code"], "invalid_args",
            "{answer}"
        );
    }
    let answer = plugin.call("buwiz.session.open", open("home", &perms, not_after));
    let result = &answer["result"]["structuredContent"];
    let cert = &result["cert"];
    assert_eq!(
        cert,
        &json!({ "v": 1, "typ": "buwiz-session", "plugin": "app.buwiz.books", "app": "home",
                 "open_id": open_id, "ks_pub": b64::encode(&[2u8; 32]), "perms_sha256": perms, "not_after": not_after })
    );
    let sig: [u8; 64] = b64::decode_exact(result["cert_sig"].as_str().unwrap(), 64)
        .unwrap()
        .try_into()
        .unwrap();
    let bytes = jcs::canonical(cert).unwrap();
    assert!(
        signing
            .verifying_key()
            .verify(bytes.as_bytes(), &Signature::from_bytes(&sig))
            .is_ok()
    );

    // an app call through the certified open answers counts and status only
    let caller = json!({ "origin": "app", "plugin": "app.buwiz.books", "app": "home", "open_id": open_id, "ts": now_ms() });
    let status = plugin.call_as("books_status", json!({}), caller);
    assert_eq!(status["result"]["isError"], false, "{status}");
    assert_eq!(
        status["result"]["structuredContent"]["status"],
        "not_set_up"
    );
    assert_eq!(status["result"]["structuredContent"]["books"], 0);
    // an open Books never certified is refused
    let forged = json!({ "origin": "app", "plugin": "app.buwiz.books", "app": "home", "open_id": b64::encode(&[3u8; 16]), "ts": now_ms() });
    let denied = plugin.call_as("books_status", json!({}), forged);
    assert_eq!(
        denied["result"]["structuredContent"]["code"],
        "permission_denied"
    );

    // /books resolves to an open of home with a ticket; the ticket's title is neutral (P3)
    let resolved = plugin.call_as(
        "books_resolve",
        json!({ "kind": "command", "name": "books", "value": "sdk-probe", "complete": false, "context": { "attached": [] } }),
        json!({ "origin": "resolve", "plugin": "app.buwiz.books", "ts": now_ms() }),
    );
    let data = &resolved["result"]["structuredContent"];
    assert_eq!(data["result"], "open");
    assert_eq!(data["app"], "books");
    let ticket = data["ticket"].as_str().unwrap().to_string();
    assert!(books_store::plumbing::is_ticket(&ticket));
    let title = plugin.call("buwiz.ticket.resolve", json!({ "ticket": ticket }));
    assert_eq!(
        title["result"]["structuredContent"],
        json!({ "valid": true, "title": "Books" })
    );
    let unknown = plugin.call(
        "buwiz.ticket.resolve",
        json!({ "ticket": "tkt_00000000000000000000000000" }),
    );
    assert_eq!(
        unknown["result"]["structuredContent"],
        json!({ "valid": false })
    );

    // close and uninstall answer ok; uninstall forgets the ticket
    assert_eq!(
        plugin.call("buwiz.session.close", json!({ "open_id": open_id }))["result"]["structuredContent"],
        json!({ "ok": true })
    );
    assert_eq!(
        plugin.call("buwiz.uninstall", json!({}))["result"]["structuredContent"],
        json!({ "ok": true })
    );
    let gone = plugin.call("buwiz.ticket.resolve", json!({ "ticket": ticket }));
    assert_eq!(
        gone["result"]["structuredContent"],
        json!({ "valid": false })
    );

    // P4: nothing on stderr, so never the seed
    let stderr = plugin.finish();
    assert!(
        !stderr.contains(&b64::encode(&seed)),
        "the seed reached stderr"
    );
    assert!(stderr.is_empty(), "stderr: {stderr}");
}

#[test]
fn a_parse_error_and_an_unknown_method_answer_json_rpc_errors() {
    let mut plugin = Plugin::spawn();
    plugin.send(&json!("not an object"));
    let mut line = String::new();
    plugin.stdout.read_line(&mut line).unwrap();
    let reply: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(reply["error"]["code"], -32600);
    writeln!(plugin.stdin, "{{not json").unwrap();
    plugin.stdin.flush().unwrap();
    line.clear();
    plugin.stdout.read_line(&mut line).unwrap();
    let reply: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(reply["error"]["code"], -32700);
    assert_eq!(
        plugin.request("nope/nope", json!({}))["error"]["code"],
        -32601
    );
    assert_eq!(plugin.request("ping", json!({}))["result"], json!({}));
    plugin.finish();
}
