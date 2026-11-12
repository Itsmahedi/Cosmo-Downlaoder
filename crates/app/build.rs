use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=../../icon.ico");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let out_dir = match env::var("OUT_DIR") {
            Ok(d) => PathBuf::from(d),
            Err(_) => return,
        };
        let res_obj = out_dir.join("app_res.o");
        let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
            Ok(d) => PathBuf::from(d),
            Err(_) => return,
        };
        let rc_file = manifest_dir.join("app.rc");

        let status = Command::new("windres")
            .current_dir(&manifest_dir)
            .args(["-i", rc_file.to_str().unwrap_or("app.rc"), "-o", res_obj.to_str().unwrap_or("app_res.o")])
            .status();

        if let Ok(s) = status {
            if s.success() {
                println!("cargo:rustc-link-arg={}", res_obj.display());
            }
        }
    }
}
