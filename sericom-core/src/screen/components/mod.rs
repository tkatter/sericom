mod cell;
mod line;
mod span;

pub use cell::Cell;
pub use line::Line;
pub use span::Span;

#[macro_export]
#[doc(hidden)]
macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),*) };
}
