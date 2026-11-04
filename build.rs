fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon.ico");

    // Build scripts run on the host, which may differ from the target. Embed the resource only
    // when the executable being built is for Windows.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/app-icon.ico")
            .compile()
            .expect("failed to embed assets/app-icon.ico in the Windows executable");
    }
}
