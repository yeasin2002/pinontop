fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=delayimp");
        println!("cargo:rustc-link-arg=/DELAYLOAD:comctl32.dll");
    }
    tauri_build::build();
}
