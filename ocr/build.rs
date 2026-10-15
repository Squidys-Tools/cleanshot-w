use std::env;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && env::var_os("CARGO_FEATURE_TESSERACT").is_some()
    {
        for library in ["advapi32", "crypt32", "iphlpapi", "secur32", "xmllite"] {
            println!("cargo:rustc-link-lib=dylib={library}");
        }
    }
}
