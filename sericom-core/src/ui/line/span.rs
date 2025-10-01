use std::ops::{Index, IndexMut};

use crossterm::style::{Attribute, Attributes, Color, Colors};

use crate::{
    configs::get_config,
    ui::{Cell, ColorState},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Span {
    pub(crate) cells: Vec<Cell>,
    pub(crate) attrs: Attributes,
    pub(crate) colors: Colors,
}

impl Default for Span {
    fn default() -> Self {
        let colors = Self::get_config_colors();
        Self {
            cells: Vec::default(),
            attrs: Attributes::default(),
            colors,
        }
    }
}

impl Span {
    fn get_config_colors() -> Colors {
        let config = get_config();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);
        Colors::new(fg, bg)
    }
    pub(crate) fn set_attrs(&mut self, attrs: Attributes) {
        self.attrs = attrs;
    }
    pub(crate) fn add_attr(&mut self, attr: Attribute) {
        self.attrs.set(attr);
    }
    pub(crate) fn reset(&mut self) {
        self.cells.iter_mut().for_each(|cell| {
            *cell = Cell::EMPTY;
        });
        self.attrs = Attributes::default();
        self.colors = Self::get_config_colors();
    }
    pub(crate) fn new_empty(width: usize) -> Self {
        let colors = Self::get_config_colors();
        Self {
            cells: vec![Cell::EMPTY; width],
            attrs: Attributes::default(),
            colors,
        }
    }
    /// Creates a new [`Span`] and reserves space for `width` of [`Cell`]s.
    ///
    /// This calls [`Vec::with_capacity()`] and does not create any [`Cell`]s.
    /// Create the [`Span`] with `colors` and/or `attrs`. If `None`, uses colors
    /// from config file ([`get_config()`]) and [`Attributes::default()`].
    pub(crate) fn reserve_new(
        width: usize,
        colors: Option<Colors>,
        attrs: Option<Attributes>,
    ) -> Self {
        let colors = colors.unwrap_or_else(Self::get_config_colors);
        let attrs = attrs.unwrap_or_default();
        Self {
            cells: Vec::with_capacity(width),
            attrs,
            colors,
        }
    }
    pub(crate) fn fill_to_width(&mut self, width: usize) {
        self.cells.resize(width, Cell::EMPTY);
    }
    pub(crate) fn shrink(&mut self) {
        let size = self.cells.len();
        self.cells.shrink_to(size);
    }
    pub(crate) fn push(&mut self, cell: Cell) {
        self.cells.push(cell);
    }
    pub(crate) fn len(&self) -> usize {
        self.cells.len()
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
