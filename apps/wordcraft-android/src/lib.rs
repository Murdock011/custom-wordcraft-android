//! WordCraft native Android host (spike). The Android entry point is `android_main`; on other
//! platforms this crate is empty.

// Off Android the modules below have no callers (only the host tests use them), so their items
// would be dead code and fail `clippy -D warnings`.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod app;
mod clip;
mod files;
mod ime;
mod speech;
