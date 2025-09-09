use super::Cell;
use std::ops::{Index, IndexMut};

/// Line is a wrapper around `Vec<Cell>` and represents a line within the [`ScreenBuffer`].
#[derive(Clone, Debug)]
pub(super) struct Line(Vec<Cell>);

impl Line {
    pub(super) fn new(width: usize) -> Self {
        Self(vec![Cell::default(); width])
    }

    pub(super) fn reset(&mut self) {
        self.0.iter_mut().for_each(|cell| *cell = Cell::default());
    }

    pub(super) fn reset_to_idx(&mut self, idx: usize) {
        self.0[..idx]
            .iter_mut()
            .for_each(|cell| *cell = Cell::default());
    }

    pub(super) fn reset_from_idx(&mut self, idx: usize) {
        self.0
            .iter_mut()
            .skip(idx)
            .for_each(|cell| *cell = Cell::default());
    }

    pub(super) fn set_char(&mut self, idx: usize, ch: char) {
        self.0[idx].character = ch;
    }

    pub(super) const fn len(&self) -> usize {
        self.0.len()
    }

    pub(super) fn clear_selection(&mut self) {
        self.0.iter_mut().for_each(|cell| cell.is_selected = false);
    }

    pub(super) fn get_cell(&self, idx: usize) -> Option<&Cell> {
        self.0.get(idx)
    }

    pub(super) fn get_mut_cell(&mut self, idx: usize) -> Option<&mut Cell> {
        self.0.get_mut(idx)
    }
}

impl IntoIterator for Line {
    type Item = Cell;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Line {
    type Item = &'a Cell;
    type IntoIter = std::slice::Iter<'a, Cell>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut Line {
    type Item = &'a mut Cell;
    type IntoIter = std::slice::IterMut<'a, Cell>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl Index<usize> for Line {
    type Output = Cell;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for Line {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}
