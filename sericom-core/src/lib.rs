#![doc(html_root_url = "https://docs.rs/sericom-core/0.3.0")]
//! `sericom-core` is the underlying library for [`sericom`](https://crates.io/crates/sericom)
//!
//! As it sits right now, this library is largely meant to be solely used by `sericom`
//! directly. Therefore, it is not intended to be used within other projects/crates.
//!
//! If other projects develop a need to use this library within their projects, please
//! create an [issue](https://github.com/tkatter/sericom) so I can become aware and work
//! towards making `sericom-core` a generalized/compatible library that is better suited
//! for use among other crates.

pub mod cli;
pub mod configs;
pub mod debug;
pub mod path_utils;
pub mod screen_buffer;
pub mod serial_actor;
