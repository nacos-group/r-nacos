#![allow(unused_must_use)]
use std::{env, path::Path};

fn main() {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let project_dir_path = Path::new(&project_dir);
    let web_dir = project_dir_path.join("target").join("rnacos-web");
    std::fs::create_dir_all(web_dir);
}