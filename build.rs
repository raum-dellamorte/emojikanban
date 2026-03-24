

fn main() {
  if std::env::var("CARGO_CFG_TARGET_OS").map(|s| s.contains("windows") ).unwrap_or(false) {
    let proj_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/deps", proj_dir);
  }
}
