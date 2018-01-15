use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
  println!(
    "cargo:rustc-env=BUILD_TIMESTAMP={:?}",
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
  );
}
