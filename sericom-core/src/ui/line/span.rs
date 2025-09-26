use std::ops::{Index, IndexMut};

use crossterm::style::{Attributes, Color, Colors};

use crate::{
    configs::get_config,
    ui::{Cell, ColorState},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Span {
    cells: Vec<Cell>,
    attrs: Attributes,
    colors: Colors,
}

impl Default for Span {
    fn default() -> Self {
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);
        Self {
            cells: Vec::default(),
            attrs: Attributes::default(),
            colors: Colors::new(fg, bg),
        }
    }
}

impl Span {
    pub(crate) fn push(&mut self, cell: Cell) {
        self.cells.push(cell);
    }
    pub(crate) fn count(&self) -> usize {
        self.cells.iter().count()
    }
    pub(crate) fn set_colors(&mut self, colors: &ColorState) {
        self.colors = colors.get_colors();
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    pub fn iter(&self) -> std::slice::Iter<'_, Cell> {
        self.cells.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Cell> {
        self.cells.iter_mut()
    }
}

impl Index<usize> for Span {
    type Output = Cell;
    fn index(&self, index: usize) -> &Self::Output {
        &self.cells[index]
    }
}

impl IndexMut<usize> for Span {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.cells[index]
    }
}
