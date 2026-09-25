//! A Dyon script exercises the encoders, regex and hashing commands through
//! the shared runtime, proving `vr_std::register` wires them into
//! `from_source_with`.

use vr_dyon::DyonRuntime;

const SCRIPT: &str = r#"
fn run() -> str {
    return base64_encode("VisualRust") +
        "|" + base64_decode("VmlzdWFsUnVzdA==") +
        "|" + url_encode("a b") +
        "|" + url_decode("a%20b") +
        "|" + regex_replace("a", "b", "banana") +
        "|" + md5("abc") +
        "|" + sha1("abc") +
        "|" + sha256("abc") +
        "|" + hmac_sha256("Jefe", "what do ya want for nothing?")
}

fn matches() -> bool {
    return regex_match("Rust", "VisualRust")
}

fn main() {}
"#;

const EXPECTED: &str = concat!(
    "VmlzdWFsUnVzdA==|VisualRust|a%20b|a b|bbnbnb",
    "|900150983cd24fb0d6963f7d28e17f72",
    "|a9993e364706816aba3e25717850c26c9cd0d89d",
    "|ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    "|5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
);

#[test]
fn dyon_script_uses_the_encoding_library() {
    let mut runtime =
        DyonRuntime::from_source_with("encoding_end_to_end.dyon", SCRIPT, vr_std::register)
            .expect("program compiles with the native signatures");
    let result: String = runtime.call_ret("run", &[]).expect("script runs");
    assert_eq!(result, EXPECTED);
    let matched: bool = runtime.call_ret("matches", &[]).expect("script runs");
    assert!(matched);
}

#[test]
fn invalid_base64_surfaces_as_a_runtime_error() {
    let source = r#"
fn run() -> str {
    return base64_decode("not base64!")
}
fn main() {}
"#;
    let mut runtime = DyonRuntime::from_source_with("encoding_bad.dyon", source, vr_std::register)
        .expect("program compiles");
    assert!(runtime.call_ret::<String>("run", &[]).is_err());
}
