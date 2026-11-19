use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const REFUSED: &str = "Sign in with a Microsoft account that owns Minecraft: Java Edition to play.";
const NOT_PREPARED: &str = "is not prepared yet";

fn home(name: &str, accounts: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("horizon-owner-gate-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let version = root.join("launcher/versions/v2026.10.27.01");
    std::fs::create_dir_all(&version).unwrap();
    std::fs::write(version.join(".installed"), "").unwrap();
    std::fs::write(root.join("launcher/accounts.json"), accounts).unwrap();
    root
}

fn launcher(root: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_horizon-launcher"));
    cmd.args(args)
        .env("HORIZON_HOME", root.join("game"))
        .env("HORIZON_LAUNCHER_HOME", root.join("launcher"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("HORIZON_LAUNCHER_NO_SELF_UPDATE", "1")
        .env("HORIZON_NO_AUDIO", "1")
        .env("HORIZON_TELEMETRY_URL", "off")
        // Memory only: the test never reads the player's keychain.
        .env("HORIZON_NO_KEYCHAIN", "1")
        .env("HORIZON_MSA_CLIENT_ID", "test-client");
    cmd
}

fn text(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr)
}

#[test]
fn an_offline_account_is_added_and_played_without_any_account() {
    let root = home("cli-offline", r#"{"accounts":[],"selected":null}"#);
    let out = launcher(&root, &["account", "offline", "Builder_1"])
        .output()
        .unwrap();
    let said = text(&out);
    assert!(out.status.success(), "{said}");
    assert!(said.contains("plays offline as Builder_1"), "{said}");
    let accounts = std::fs::read_to_string(root.join("launcher/accounts.json")).unwrap();
    assert!(accounts.contains("Builder_1"), "{accounts}");
    let out = launcher(&root, &["play"]).output().unwrap();
    let said = text(&out);
    assert!(!out.status.success(), "{said}");
    assert!(said.contains(NOT_PREPARED), "{said}");
    assert!(!said.contains(REFUSED), "{said}");
    assert!(!said.contains("account sign-in"), "{said}");
    assert!(!root.join("game/sessions").exists());
    let accounts = std::fs::read_to_string(root.join("launcher/accounts.json")).unwrap();
    assert!(accounts.contains("Builder_1"), "{accounts}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn prepare_download_downloads_for_an_offline_name() {
    let root = home("prepare-download", r#"{"accounts":[],"selected":null}"#);
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../research-lock.json"),
        root.join("launcher/versions/v2026.10.27.01/research-lock.json"),
    )
    .unwrap();
    // The version's own preparer, as a release package ships it. Its work is not this
    // test's subject: only that Mojang's public files download for an offline name,
    // with no account at all and no sign-in refusal anywhere.
    std::fs::create_dir_all(root.join("launcher/versions/v2026.10.27.01/tools")).unwrap();
    std::fs::write(
        root.join("launcher/versions/v2026.10.27.01/tools/prepare_native_client.py"),
        "# A stub: this test checks the account gate, not the preparation itself.\n",
    )
    .unwrap();
    let out = launcher(
        &root,
        &["prepare", "--download", "--version", "v2026.10.27.01"],
    )
    .current_dir(&root)
    .env("HOME", &root)
    .env("HORIZON_LAUNCHER_TRASH", root.join("trash"))
    .env("HORIZON_LAUNCHER_CLIPBOARD", root.join("clipboard"))
    .env("HORIZON_LAUNCHER_DESKTOP", root.join("desktop"))
    .output()
    .unwrap();
    let said = text(&out);
    assert!(out.status.success(), "{said}");
    assert!(said.contains("Downloading 26.3 from Mojang"), "{said}");
    assert!(said.contains("is prepared."), "{said}");
    assert!(!said.contains("Sign in with a Microsoft account"), "{said}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_selected_microsoft_account_with_no_owner_still_meets_the_gate() {
    let root = home(
        "cli-gate",
        r#"{"accounts":[{"uuid":"abc123","name":"Notch"}],"selected":"abc123"}"#,
    );
    let out = launcher(&root, &["play"]).output().unwrap();
    let said = text(&out);
    assert!(!out.status.success(), "{said}");
    assert!(said.contains(REFUSED), "{said}");
    assert!(said.contains("horizon-launcher account sign-in"), "{said}");
    assert!(!root.join("game/sessions").exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_mcp_server_plays_an_offline_account_past_the_gate() {
    let root = home("mcp", r#"{"accounts":[],"selected":null}"#);
    let mut child = launcher(&root, &["mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let requests = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"play","arguments":{}}}"#,
    ];
    let mut stdin = child.stdin.take().unwrap();
    for r in requests {
        writeln!(stdin, "{r}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    let reply = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .find(|v| v["id"] == 2)
        .expect("a reply to play");
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    let said = reply["result"]["content"][0]["text"].as_str().unwrap();
    assert!(said.contains(NOT_PREPARED), "{said}");
    assert!(!said.contains(REFUSED), "{said}");
    let _ = std::fs::remove_dir_all(&root);
}
