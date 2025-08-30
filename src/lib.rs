pub mod configs;
pub mod debug;
pub mod screen_buffer;
pub mod serial_actor;

mod macros {
    //! This module holds generic macros that are used throughout sericom.

    /// Takes a [`&Path`][std::path::Path] and first checks whether it exists or if it is a
    /// directory. If it doesn't exist or is not a directory, it will create
    /// the directory recursively; creating the necessary parent directories.
    #[macro_export]
    macro_rules! create_recursive {
        ($path:expr) => {
            let create_recursive_dir = |p: &std::path::Path| {
                if !p.exists() || !p.is_dir() {
                    let mut builder = std::fs::DirBuilder::new();
                    builder.recursive(true);
                    builder.create(p).expect("Recursive mode won't panic");
                }
            };

            create_recursive_dir($path)
        };
    }
}
