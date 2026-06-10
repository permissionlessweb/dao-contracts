/// Generate SUITE_API.md — the full DAO DAO contract API reference.
///
/// Usage:
///   cargo run -p dao-testing --bin suite_api
///   cargo run -p dao-testing --bin suite_api -- --out path/to/output.md
fn main() {
    let ws = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let out = std::env::args()
        .nth(1)
        .filter(|a| a == "--out")
        .and_then(|_| std::env::args().nth(2))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| ws.join("SUITE_API.md"));

    // // let md = dao_testing::suite::generate_api_markdown(ws);
    // std::fs::write(&out, &md).expect("failed to write");
    // eprintln!("{} bytes → {}", md.len(), out.display());
}
