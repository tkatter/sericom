use std::ops::{Index, IndexMut};

use crossterm::style::{Attributes, SetAttributes, SetColors};

use crate::screen::ColorState;

use super::Span;

/// Line is a wrapper around [`Vec<Cell>`] and represents a line within the [`ScreenBuffer`][`super::super::ScreenBuffer`].
#[derive(Clone, Eq, PartialEq, Default)]
pub struct Line(pub Vec<Span>);

impl Line {
    /// Create a new [`Line`] with a single [`Span`] with the length of `width`.
    ///
    /// The [`Span`] is filled with [`Cell::EMPTY`], default [`Attributes`] and [`Colors`].
    #[must_use]
    pub fn new_empty(width: usize) -> Self {
        Self(vec![Span::new_empty(width, None, None); 1])
    }

    /// Util function to return the length of `self`.
    ///
    /// **Note:** This returns the length as in the number of [`Span`]s _not_ [`Cell`]s.
    #[must_use]
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterates over the [`Cell`]s and resets their selected state.
    pub fn clear_selection(&mut self) {
        self.0
            .iter_mut()
            .flatten()
            .for_each(|cell| cell.is_selected = false);
    }

    /// Returns a reference to the [`Span`] at `idx`.
    #[must_use]
    pub fn get_span(&self, idx: usize) -> Option<&Span> {
        self.0.get(idx)
    }

    /// Returns a mutable reference to the [`Span`] at `idx`.
    pub fn get_mut_span(&mut self, idx: usize) -> Option<&mut Span> {
        self.0.get_mut(idx)
    }

    /// Returns the index of the [`Span`] and the offset within the [`Span`] for `col`
    #[must_use]
    pub fn span_at_col(&self, col: usize) -> (usize, usize) {
        if self.0.len() == 1 {
            return (0, col);
        }

        let mut acc = 0;
        for (idx, span) in self.0.iter().enumerate() {
            let end = acc + span.len();
            if col < end {
                return (idx, col - acc);
            }
            acc = end;
        }
        (self.0.len().saturating_sub(1), self.0.last().unwrap().len())
    }

    /// The total number of [`Cell`]s in `self`, where [`Cell`] != ' '.
    ///
    /// **Note:** This number includes whitespace between words since that is
    /// generally the desired behavior.
    #[must_use]
    pub fn filled_cells(&self) -> usize {
        let last = self.last_filled_idx() + 1;
        if self.num_cells() == last { 0 } else { last }
    }

    /// The total number of [`Cell`]s in `self`.
    #[must_use]
    pub fn num_cells(&self) -> usize {
        self.iter().flatten().count()
    }

    /// The index of the last [`Cell`] in `self` where [`Cell`] != ' ' (whitespace).
    #[must_use]
    pub fn last_filled_idx(&self) -> usize {
        if self.all_whitespace() {
            return 0;
        }

        let len = self.num_cells();
        let p = self
            .iter()
            .flatten()
            .rev()
            .position(|c| c.character != b' ')
            .unwrap_or(0);
        if len - p == len { len - 1 } else { len - p }
    }

    fn all_whitespace(&self) -> bool {
        for c in self.iter().flatten() {
            if **c != b' ' {
                return false;
            }
        }
        true
    }

    /// Whether [`Line`] contains zero _[`Span`]s_.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Span> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Span> {
        self.0.iter_mut()
    }

    /// Push a [`Span`] to `self`.
    pub fn push(&mut self, span: Span) {
        self.0.push(span);
    }

    pub fn ascii_bytes(&self) -> impl Iterator<Item = u8> + '_ {
        use std::ops::Deref;

        let end = self.last_filled_idx();
        self.iter()
            .flatten()
            .take(end + 1)
            .map(Deref::deref)
            .copied()
    }

    /// Splits the [`Span`] at `col` and applies `colors` && `attrs` to the new [`Span`].
    ///
    /// If there is only a single [`Span`] in `self` and [`Self::num_filled_cells()`] == 0
    /// then this will not split the [`Span`] or create a new one. Instead it will
    /// just apply the [`ColorState`] and [`Attributes`] to that [`Span`].
    pub fn split_spans(&mut self, colors: &ColorState, attrs: Attributes, col: usize) {
        // Handles the case where an ESC[ is the first input for an empty line
        if self.len() == 1
            && self.filled_cells() == 0
            && let Some(span) = self.get_mut_span(0)
        {
            span.set_colors(colors);
            span.set_attrs(attrs);
            tracing::trace!(target: "line::split::first", ?span);
            return;
        }

        let (span_idx, offset) = self.span_at_col(col);
        let Some(span) = self.get_mut_span(span_idx) else {
            tracing::trace!(target: "line::split", %col, %span_idx, "FAILED TO GET SPAN");
            return;
        };
        tracing::trace!(target: "line::split::rest", ?span);
        let mut new = span.split_at(offset);
        new.set_colors(colors);
        new.set_attrs(attrs);
        self.push(new);
    }
}

impl IntoIterator for Line {
    type Item = Span;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Line {
    type Item = &'a Span;
    type IntoIter = std::slice::Iter<'a, Span>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Line {
    type Item = &'a mut Span;
    type IntoIter = std::slice::IterMut<'a, Span>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl Index<usize> for Line {
    type Output = Span;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Line {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl crossterm::Command for Line {
    #[cfg(windows)]
    fn execute_winapi(&self) -> Result<(), std::io::Error> {
        todo!()
    }

    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        let mut spans = self.iter();
        let Some(first) = spans.next() else {
            return Ok(());
        };

        let mut colors = first.colors;
        let mut attrs = first.attrs;

        SetColors(colors).write_ansi(f)?;
        SetAttributes(attrs).write_ansi(f)?;
        first.write_ansi(f)?;

        for span in spans {
            if span.colors != colors {
                SetColors(span.colors).write_ansi(f)?;
                colors = span.colors;
            }

            if span.attrs != attrs {
                SetAttributes(span.attrs).write_ansi(f)?;
                attrs = span.attrs;
            }

            span.write_ansi(f)?;
        }

        Ok(())
    }
}

impl std::fmt::Debug for Line {
    /// Prints "Line [cells: {#}] ( {spans} )"
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        let mut s = format!("Line [cells: {}] ( ", self.num_cells());
        for span in self {
            let _ = write!(s, "{span:?}");
        }
        s.push_str(" )");
        f.write_str(&s)
    }
}
