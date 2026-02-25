

fn main() {
  let proj_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
  println!("cargo:rustc-link-search=native={}/deps", proj_dir);
}
