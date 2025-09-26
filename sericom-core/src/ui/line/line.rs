use std::ops::{Index, IndexMut};

use crate::ui::Span;

/// Line is a wrapper around [`Vec<Cell>`] and represents a line within the [`ScreenBuffer`][`super::ScreenBuffer`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Line(Vec<Span>);

impl Line {
    /// Create a new line with the length/size of `width`.
    ///
    /// Filled with `span`.
    #[must_use]
    pub fn new(width: usize, span: Span) -> Self {
        Self(vec![span; width])
    }

    /// Create a new line with the length/size of `width`.
    ///
    /// Filled with [`Cell::default()`].
    #[must_use]
    pub fn new_default(width: usize) -> Self {
        Self(vec![Span::default(); width])
    }

    /// Create a new line with the length/size of `width`.
    ///
    /// Filled with [`Cell::EMPTY`].
    #[must_use]
    pub fn new_empty(width: usize) -> Self {
        Self(vec![Span::default(); width])
    }

    /// Iterates over all the [`Cell`]s within the line and sets them to [`Cell::default()`].
    pub fn reset(&mut self) {
        self.0.iter_mut().for_each(|cell| *cell = Cell::default());
    }

    /// Iterates over the [`Cell`]s to index `idx` within [`Self`]
    /// and sets them to [`Cell::default()`].
    pub fn reset_to(&mut self, idx: usize) {
        self.0[..idx]
            .iter_mut()
            .for_each(|cell| *cell = Cell::default());
    }

    /// Iterates over the [`Cell`]s from index `idx` within [`Self`]
    /// to the end of [`Self`] and sets them to [`Cell::default()`].
    pub fn reset_from(&mut self, idx: usize) {
        self.0
            .iter_mut()
            .skip(idx)
            .for_each(|cell| *cell = Cell::default());
    }

    /// Sets the character in [`Cell`] at [`Self`]\[`idx`\] to `ch`.
    pub fn set_char(&mut self, idx: usize, ch: char) {
        self.0[idx].character = ch;
    }

    /// Util function to return the length of [`Self`].
    #[must_use]
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterates over the [`Cell`]s and resets their selected state.
    pub fn clear_selection(&mut self) {
        self.0
            .iter_mut()
            .for_each(|span| span.iter_mut().for_each(|cell| cell.is_selected = false));
    }

    /// Returns a reference to [`Cell`] at `idx`.
    #[must_use]
    pub fn get_span(&self, idx: usize) -> Option<&Span> {
        self.0.get(idx)
    }

    /// Returns a mutable reference to [`Cell`] at `idx`.
    pub fn get_mut_span(&mut self, idx: usize) -> Option<&mut Span> {
        self.0.get_mut(idx)
    }

    pub fn count_cells(&self) -> usize {
        let mut num_cells = 0;
        self.0.iter().for_each(|span| {
            num_cells += span.count();
        });
        num_cells
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Span> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Span> {
        self.0.iter_mut()
    }

    pub fn push(&mut self, span: Span) {
        self.0.push(span);
    }
}

impl Default for Line {
    fn default() -> Self {
        Self(Default::default())
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
