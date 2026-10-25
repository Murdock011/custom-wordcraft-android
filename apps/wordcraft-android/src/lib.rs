//! WordCraft native Android host (spike). The Android entry point is `android_main`; on other
//! platforms this crate is empty.

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
mod app;
mod clip;
mod files;
mod ime;
