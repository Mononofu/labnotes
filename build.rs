use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
  let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
  println!(
    "cargo:rustc-env=BUILD_TIMESTAMP={}{:09}",
    d.as_secs(),
    d.subsec_nanos()
  );
}
