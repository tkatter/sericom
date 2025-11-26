use std::{
    fmt::Formatter,
    ops::{Deref, Index, IndexMut},
};

use crossterm::{
    Command,
    style::{Attribute, Attributes, Color, Colors, ContentStyle, StyledContent, Stylize},
};

use crate::{configs::get_config, screen::Cell, screen::process::ColorState};

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

    pub(crate) const fn set_attrs(&mut self, attrs: Attributes) {
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

    pub(crate) fn new_empty_colors(
        width: usize,
        colors: Option<Colors>,
        attrs: Option<Attributes>,
    ) -> Self {
        let colors = colors.unwrap_or_else(Self::get_config_colors);
        let attrs = attrs.unwrap_or_default();
        Self {
            cells: vec![Cell::EMPTY; width],
            attrs,
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

    pub(crate) const fn len(&self) -> usize {
        self.cells.len()
    }

    pub(crate) const fn set_colors(&mut self, colors: &ColorState) {
        self.colors = colors.get_colors();
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Cell> {
        self.cells.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Cell> {
        self.cells.iter_mut()
    }

    /// Returns the number of [`Cell`]s in a [`Span`] that are not [`Cell::EMPTY`]
    #[must_use]
    pub fn num_filled_cells(&self) -> usize {
        self.cells
            .iter()
            .filter(|&cell| *cell != Cell::EMPTY)
            .count()
    }

    const fn content_style(&self) -> ContentStyle {
        ContentStyle {
            foreground_color: self.colors.foreground,
            background_color: self.colors.background,
            underline_color: None,
            attributes: self.attrs,
        }
    }

    pub(crate) const fn colors(&self) -> Colors {
        self.colors
    }

    pub(crate) const fn attrs(&self) -> Attributes {
        self.attrs
    }

    pub(crate) fn styled(&self) -> StyledContent<String> {
        let s = String::from_iter(&self.cells);
        self.content_style().apply(s)
    }
}

impl Command for Span {
    fn write_ansi(&self, f: &mut impl std::fmt::Write) -> std::fmt::Result {
        f.write_str(&String::from_iter(&self.cells))
    }
}

impl IntoIterator for Span {
    type Item = Cell;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.cells.into_iter()
    }
}

impl<'a> IntoIterator for &'a Span {
    type Item = &'a Cell;
    type IntoIter = std::slice::Iter<'a, Cell>;

    fn into_iter(self) -> Self::IntoIter {
        self.cells.iter()
    }
}

impl<'a> IntoIterator for &'a mut Span {
    type Item = &'a mut Cell;
    type IntoIter = std::slice::IterMut<'a, Cell>;

    fn into_iter(self) -> Self::IntoIter {
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

impl FromIterator<char> for Span {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let colors = Self::get_config_colors();
        Self {
            cells: iter.into_iter().map(Cell::from).collect(),
            colors,
            attrs: Attributes::default(),
        }
    }
}

impl<'a> FromIterator<&'a Cell> for std::string::String {
    fn from_iter<T: IntoIterator<Item = &'a Cell>>(iter: T) -> Self {
        iter.into_iter().map(Deref::deref).collect::<Self>()
    }
}
