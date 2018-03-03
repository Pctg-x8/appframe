
#[cfg(target_os = "macos")]
fn main()
{
    println!("cargo:rustc-link-search=framework={}/MoltenVK/macOS", env!("VK_SDK_PATH"));
    println!("cargo:rustc-link-search=framework={}/macOS/Frameworks", env!("VK_SDK_PATH"));
    println!("cargo:rustc-link-lib=c++");
    println!("cargo:rustc-link-lib=framework=IOSurface");
    println!("cargo:rustc-link-lib=framework=IOKit");
    println!("cargo:rustc-link-lib=framework=vulkan");
}
#[cfg(windows)]
fn main()
{
    println!("cargo:rustc-link-search=static={}/Lib", env!("VK_SDK_PATH"));
}
#[cfg(not(all(target_os = "macos", windows)))]
fn main() {}
