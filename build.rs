fn main() {
    println!("cargo:rerun-if-env-changed=FONTCORE_TEST_FONTS");
    println!("cargo:rustc-check-cfg=cfg(fontcore_external_corpus)");
    if std::env::var_os("FONTCORE_TEST_FONTS").is_some() {
        println!("cargo:rustc-cfg=fontcore_external_corpus");
    }
}
