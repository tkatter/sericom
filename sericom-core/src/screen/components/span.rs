use std::ops::{Index, IndexMut};

use crossterm::{
    Command,
    style::{Attribute, Attributes, Color, Colors},
};

use crate::{
    configs::get_config,
    screen::{Cell, process::ColorState},
};

#[derive(Clone, Eq, PartialEq)]
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
    /// Splits `self` [0, idx) and returns a new [`Span`] [idx, len).
    ///
    /// The returned [`Span`] has the same properties (colors and attributes) as `self`.
    pub(crate) fn split_at(&mut self, idx: usize) -> Self {
        Self {
            cells: self.cells.split_off(idx),
            colors: self.colors,
            attrs: self.attrs,
        }
    }

    /// The index of the last [`Cell`] in `self` that is not whitespace.
    #[must_use]
    pub fn last_filled_idx(&self) -> usize {
        if self.all_whitespace() {
            return 0;
        }

        let len = self.len();
        let p = self
            .iter()
            .rev()
            .position(|c| c.character != b' ')
            .unwrap_or(0);
        if len - p == len { len - 1 } else { len - p }
    }

    fn all_whitespace(&self) -> bool {
        for c in &self.cells {
            if **c != b' ' {
                return false;
            }
        }
        true
    }

    fn get_config_colors() -> Colors {
        let config = get_config().unwrap();
        let fg = Color::from(&config.appearance.fg);
        let bg = Color::from(&config.appearance.bg);
        drop(config);
        Colors::new(fg, bg)
    }

    /// Set the [`Attributes`] for `self`.
    pub const fn set_attrs(&mut self, attrs: Attributes) {
        self.attrs = attrs;
    }

    /// Add an [`Attribute`] to [`Self`], if already set this does nothing.
    pub fn add_attr(&mut self, attr: Attribute) {
        self.attrs.set(attr);
    }

    /// Reset all [`Cell`]s within `self`.
    ///
    /// Sets every [`Cell`] in [`Self`] to [`Cell::EMPTY`], sets [`Attributes`]
    /// to [`Attributes::default()`] and [`Colors`] to those from [`Config::appearance`].
    ///
    /// [`Config::appearance`]: `crate::configs::Config::appearance`
    pub(crate) fn reset(&mut self) {
        self.cells.iter_mut().for_each(|cell| {
            *cell = Cell::EMPTY;
        });
        self.attrs = Attributes::default();
        self.colors = Self::get_config_colors();
    }

    /// Creates a new [`Span`] filled with [`Cell::EMPTY`] to `width`.
    ///
    /// Create the [`Span`] with [`Colors`] and/or [`Attributes`]. If `None`,
    /// uses colors from config file ([`get_config()`]) and [`Attributes::default()`].
    pub fn new_empty(width: usize, colors: Option<Colors>, attrs: Option<Attributes>) -> Self {
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

    /// Resizes `self` to `width` with [`Cell::EMPTY`].
    pub(crate) fn fill_to_width(&mut self, width: usize) {
        self.cells.resize(width, Cell::EMPTY);
    }

    /// Shrinks `self` to the last filled [`Cell`], think [`str::trim_end()`].
    ///
    /// Finds the last [`Cell`] in `self` that is not whitespace and truncates
    /// `self` to that index, dropping any [`Cell`]s past it, and then calls
    /// [`shrink_to()`] that same index to then truncate `self`s capacity.
    ///
    /// [`shrink_to()`]: `Vec::shrink_to()`
    pub fn shrink(&mut self) {
        let size = self.last_filled_idx();
        self.cells.truncate(size);
        self.cells.shrink_to(size);
    }

    /// Push a [`Cell`] to `self`.
    pub(crate) fn push(&mut self, cell: Cell) {
        self.cells.push(cell);
    }

    /// Length of [`Cell`]s in `self`.
    pub(crate) const fn len(&self) -> usize {
        self.cells.len()
    }

    /// Set the [`Colors`] for `self`.
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

    pub(crate) const fn colors(&self) -> Colors {
        self.colors
    }

    pub(crate) const fn attrs(&self) -> Attributes {
        self.attrs
    }
}

impl Command for Span {
    #[cfg(windows)]
    fn execute_winapi(&self) -> Result<(), std::io::Error> {
        todo!()
    }

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

impl FromIterator<Cell> for Span {
    fn from_iter<T: IntoIterator<Item = Cell>>(iter: T) -> Self {
        let colors = Self::get_config_colors();
        Self {
            cells: iter.into_iter().collect(),
            colors,
            attrs: Attributes::default(),
        }
    }
}

impl std::fmt::Debug for Span {
    /// Prints "Span[fg: {:?}, bg: {:?}, attrs: {:?}, len: {}, cap: {}] ( {cells} )"
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = format!(
            "Span[fg: {:?}, bg: {:?}, attrs: {:?}, len: {}, cap: {}] ( ",
            self.colors.foreground.unwrap(),
            self.colors.background.unwrap(),
            self.attrs,
            self.cells.len(),
            self.cells.capacity()
        );
        s.extend(self.cells.iter().map(|c| c.character as char));
        s.push(')');
        f.write_str(&s)
    }
}
