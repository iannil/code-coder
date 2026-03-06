#[cfg(feature = "napi-bindings")]
fn main() {
    napi_build::setup();
}

#[cfg(not(feature = "napi-bindings"))]
fn main() {
    // No-op when NAPI bindings are not enabled
}
