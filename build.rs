fn main() {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    println!("cargo:rustc-link-search=./libs/linux-x64");

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    println!("cargo:rustc-link-search=./libs/win-x64");

    #[cfg(all(target_os = "macos", target_arch = "arm"))]
    println!("cargo:rustc-link-search=./libs/mac-arm64");
}
