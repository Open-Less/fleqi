fn main() {
    tauri_build::build();

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        println!("cargo:rerun-if-changed=native/Package.swift");
        println!("cargo:rerun-if-changed=native/Sources/FleqiGlass");
        swift_rs::SwiftLinker::new("15.0")
            .with_package("FleqiGlass", "native")
            .link();
    }
}
