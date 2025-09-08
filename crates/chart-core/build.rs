// File: crates/chart-core/build.rs
// Summary: Build script to link required Windows system libraries for Skia/ICU.

fn main() {
    #[cfg(target_os = "windows")]
    {
        // Needed for RegOpenKeyExW, RegQueryInfoKeyW, etc.
        println!("cargo:rustc-link-lib=advapi32");
    }
}
