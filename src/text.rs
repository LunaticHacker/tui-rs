//! Primitives for styled text.
//!
//! A terminal UI is at its root a lot of strings. In order to make it accessible and stylish,
//! those strings may be associated to a set of styles. `tui` has three ways to represent them:
//! - A single line string where all graphemes have the same style is represented by a [`Span`].
//! - A single line string where each grapheme may have its own style is represented by [`Spans`].
//! - A multiple line string where each grapheme may have its own style is represented by a
//! [`Text`].
//!
//! These types form a hierarchy: [`Spans`] is a collection of [`Span`] and each line of [`Text`]
//! is a [`Spans`].
//!
//! Keep it mind that a lot of widgets will use those types to advertise what kind of string is
//! supported for their properties. Moreover, `tui` provides convenient `From` implementations so
//! that you can start by using simple `String` or `&str` and then promote them to the previous
//! primitives when you need additional styling capabilities.
//!
//! For example, for the [`crate::widgets::Block`] widget, all the following calls are valid to set
//! its `title` property (which is a [`Spans`] under the hood):
//!
//! ```rust
//! # use tui::widgets::Block;
//! # use tui::text::{Span, Spans};
//! # use tui::style::{Color, Style};
//! // A simple string with no styling.
//! // Converted to Spans(vec![
//! //   Span { content: Cow::Borrowed("My title"), style: Style { .. } }
//! // ])
//! let block = Block::default().title("My title");
//!
//! // A simple string with a unique style.
//! // Converted to Spans(vec![
//! //   Span { content: Cow::Borrowed("My title"), style: Style { fg: Some(Color::Yellow), .. }
//! // ])
//! let block = Block::default().title(
//!     Span::styled("My title", Style::default().fg(Color::Yellow))
//! );
//!
//! // A string with multiple styles.
//! // Converted to Spans(vec![
//! //   Span { content: Cow::Borrowed("My"), style: Style { fg: Some(Color::Yellow), .. } },
//! //   Span { content: Cow::Borrowed(" title"), .. }
//! // ])
//! let block = Block::default().title(vec![
//!     Span::styled("My", Style::default().fg(Color::Yellow)),
//!     Span::raw(" title"),
//! ]);
//! ```
use crate::style::Style;
use std::borrow::Cow;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const NBSP: &str = "\u{00a0}";

/// A grapheme associated to a style.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledGrapheme<'a> {
    pub symbol: &'a str,
    pub style: Style,
}

/// A string where all graphemes have the same style.
#[derive(Debug, Clone, PartialEq)]
pub struct Span<'a> {
    pub content: Cow<'a, str>,
    pub style: Style,
}

impl<'a> Span<'a> {
    /// Create a span with no style.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::text::Span;
    /// Span::raw("My text");
    /// Span::raw(String::from("My text"));
    /// ```
    pub fn raw<T>(content: T) -> Span<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Span {
            content: content.into(),
            style: Style::default(),
        }
    }

    /// Create a span with a style.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use tui::text::Span;
    /// # use tui::style::{Color, Modifier, Style};
    /// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
    /// Span::styled("My text", style);
    /// Span::styled(String::from("My text"), style);
    /// ```
    pub fn styled<T>(content: T, style: Style) -> Span<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Span {
            content: content.into(),
            style,
        }
    }

    /// Returns the width of the content held by this span.
    pub fn width(&self) -> usize {
        self.content.width()
    }

    /// Returns an iterator over the graphemes held by this span.
    ///
    /// `base_style` is the [`Style`] that will be patched with each grapheme [`Style`] to get
    /// the resulting [`Style`].
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::text::{Span, StyledGrapheme};
    /// # use tui::style::{Color, Modifier, Style};
    /// # use std::iter::Iterator;
    /// let style = Style::default().fg(Color::Yellow);
    /// let span = Span::styled("Text", style);
    /// let style = Style::default().fg(Color::Green).bg(Color::Black);
    /// let styled_graphemes = span.styled_graphemes(style);
    /// assert_eq!(
    ///     vec![
    ///         StyledGrapheme {
    ///             symbol: "T",
    ///             style: Style {
    ///                 fg: Some(Color::Yellow),
    ///                 bg: Some(Color::Black),
    ///                 add_modifier: Modifier::empty(),
    ///                 sub_modifier: Modifier::empty(),
    ///             },
    ///         },
    ///         StyledGrapheme {
    ///             symbol: "e",
    ///             style: Style {
    ///                 fg: Some(Color::Yellow),
    ///                 bg: Some(Color::Black),
    ///                 add_modifier: Modifier::empty(),
    ///                 sub_modifier: Modifier::empty(),
    ///             },
    ///         },
    ///         StyledGrapheme {
    ///             symbol: "x",
    ///             style: Style {
    ///                 fg: Some(Color::Yellow),
    ///                 bg: Some(Color::Black),
    ///                 add_modifier: Modifier::empty(),
    ///                 sub_modifier: Modifier::empty(),
    ///             },
    ///         },
    ///         StyledGrapheme {
    ///             symbol: "t",
    ///             style: Style {
    ///                 fg: Some(Color::Yellow),
    ///                 bg: Some(Color::Black),
    ///                 add_modifier: Modifier::empty(),
    ///                 sub_modifier: Modifier::empty(),
    ///             },
    ///         },
    ///     ],
    ///     styled_graphemes.collect::<Vec<StyledGrapheme>>()
    /// );
    /// ```
    pub fn styled_graphemes(
        &'a self,
        base_style: Style,
    ) -> impl Iterator<Item = StyledGrapheme<'a>> {
        UnicodeSegmentation::graphemes(self.content.as_ref(), true)
            .map(move |g| StyledGrapheme {
                symbol: g,
                style: base_style.patch(self.style),
            })
            .filter(|s| s.symbol != "\n")
    }

    fn split_at_in_place(&mut self, mid: usize) -> Span<'a> {
        let content = match self.content {
            Cow::Owned(ref mut s) => {
                let start = s.char_indices().map(|(i, _)| i).nth(mid).unwrap();
                let s2 = s[start..].to_string();
                s.truncate(start);
                Cow::Owned(s2)
            }
            Cow::Borrowed(s) => {
                let (s1, s2) = s.split_at(mid);
                self.content = Cow::Borrowed(s1);
                Cow::Borrowed(s2)
            }
        };
        Span {
            content,
            style: self.style,
        }
    }

    fn trim_start(&mut self) {
        self.content = Cow::Owned(String::from(self.content.trim_start()));
    }
}

impl<'a> From<String> for Span<'a> {
    fn from(s: String) -> Span<'a> {
        Span::raw(s)
    }
}

impl<'a> From<&'a str> for Span<'a> {
    fn from(s: &'a str) -> Span<'a> {
        Span::raw(s)
    }
}

/// A string composed of clusters of graphemes, each with their own style.
#[derive(Debug, Clone, PartialEq)]
pub struct Spans<'a>(pub Vec<Span<'a>>);

impl<'a> Default for Spans<'a> {
    fn default() -> Spans<'a> {
        Spans(Vec::new())
    }
}

impl<'a> Spans<'a> {
    /// Returns the width of the underlying string.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::text::{Span, Spans};
    /// # use tui::style::{Color, Style};
    /// let spans = Spans::from(vec![
    ///     Span::styled("My", Style::default().fg(Color::Yellow)),
    ///     Span::raw(" text"),
    /// ]);
    /// assert_eq!(7, spans.width());
    /// ```
    pub fn width(&self) -> usize {
        self.0.iter().map(Span::width).sum()
    }
}

impl<'a> From<String> for Spans<'a> {
    fn from(s: String) -> Spans<'a> {
        Spans(vec![Span::from(s)])
    }
}

impl<'a> From<&'a str> for Spans<'a> {
    fn from(s: &'a str) -> Spans<'a> {
        Spans(vec![Span::from(s)])
    }
}

impl<'a> From<Vec<Span<'a>>> for Spans<'a> {
    fn from(spans: Vec<Span<'a>>) -> Spans<'a> {
        Spans(spans)
    }
}

impl<'a> From<Span<'a>> for Spans<'a> {
    fn from(span: Span<'a>) -> Spans<'a> {
        Spans(vec![span])
    }
}

impl<'a> From<Spans<'a>> for String {
    fn from(line: Spans<'a>) -> String {
        line.0.iter().fold(String::new(), |mut acc, s| {
            acc.push_str(s.content.as_ref());
            acc
        })
    }
}

impl<'a> IntoIterator for Spans<'a> {
    type Item = Span<'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A string split over multiple lines where each line is composed of several clusters, each with
/// their own style.
///
/// A [`Text`], like a [`Span`], can be constructed using one of the many `From` implementations
/// or via the [`Text::raw`] and [`Text::styled`] methods. Helpfully, [`Text`] also implements
/// [`core::iter::Extend`] which enables the concatenation of several [`Text`] blocks.
///
/// ```rust
/// # use tui::text::Text;
/// # use tui::style::{Color, Modifier, Style};
/// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
///
/// // An initial two lines of `Text` built from a `&str`
/// let mut text = Text::from("The first line\nThe second line");
/// assert_eq!(2, text.height());
///
/// // Adding two more unstyled lines
/// text.extend(Text::raw("These are two\nmore lines!"));
/// assert_eq!(4, text.height());
///
/// // Adding a final two styled lines
/// text.extend(Text::styled("Some more lines\nnow with more style!", style));
/// assert_eq!(6, text.height());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Text<'a> {
    pub lines: Vec<Spans<'a>>,
}

impl<'a> Default for Text<'a> {
    fn default() -> Text<'a> {
        Text { lines: Vec::new() }
    }
}

impl<'a> Text<'a> {
    /// Create some text (potentially multiple lines) with no style.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use tui::text::Text;
    /// Text::raw("The first line\nThe second line");
    /// Text::raw(String::from("The first line\nThe second line"));
    /// ```
    pub fn raw<T>(content: T) -> Text<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Text {
            lines: match content.into() {
                Cow::Borrowed(s) => s.lines().map(Spans::from).collect(),
                Cow::Owned(s) => s.lines().map(|l| Spans::from(l.to_owned())).collect(),
            },
        }
    }

    /// Create some text (potentially multiple lines) with a style.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use tui::text::Text;
    /// # use tui::style::{Color, Modifier, Style};
    /// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
    /// Text::styled("The first line\nThe second line", style);
    /// Text::styled(String::from("The first line\nThe second line"), style);
    /// ```
    pub fn styled<T>(content: T, style: Style) -> Text<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let mut text = Text::raw(content);
        text.patch_style(style);
        text
    }

    /// Returns the max width of all the lines.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use tui::text::Text;
    /// let text = Text::from("The first line\nThe second line");
    /// assert_eq!(15, text.width());
    /// ```
    pub fn width(&self) -> usize {
        self.lines
            .iter()
            .map(Spans::width)
            .max()
            .unwrap_or_default()
    }

    /// Returns the height.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use tui::text::Text;
    /// let text = Text::from("The first line\nThe second line");
    /// assert_eq!(2, text.height());
    /// ```
    pub fn height(&self) -> usize {
        self.lines.len()
    }

    /// Apply a new style to existing text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use tui::text::Text;
    /// # use tui::style::{Color, Modifier, Style};
    /// let style = Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC);
    /// let mut raw_text = Text::raw("The first line\nThe second line");
    /// let styled_text = Text::styled(String::from("The first line\nThe second line"), style);
    /// assert_ne!(raw_text, styled_text);
    ///
    /// raw_text.patch_style(style);
    /// assert_eq!(raw_text, styled_text);
    /// ```
    pub fn patch_style(&mut self, style: Style) {
        for line in &mut self.lines {
            for span in &mut line.0 {
                span.style = span.style.patch(style);
            }
        }
    }
}

impl<'a> From<String> for Text<'a> {
    fn from(s: String) -> Text<'a> {
        Text::raw(s)
    }
}

impl<'a> From<&'a str> for Text<'a> {
    fn from(s: &'a str) -> Text<'a> {
        Text::raw(s)
    }
}

impl<'a> From<Cow<'a, str>> for Text<'a> {
    fn from(s: Cow<'a, str>) -> Text<'a> {
        Text::raw(s)
    }
}

impl<'a> From<Span<'a>> for Text<'a> {
    fn from(span: Span<'a>) -> Text<'a> {
        Text {
            lines: vec![Spans::from(span)],
        }
    }
}

impl<'a> From<Spans<'a>> for Text<'a> {
    fn from(spans: Spans<'a>) -> Text<'a> {
        Text { lines: vec![spans] }
    }
}

impl<'a> From<Vec<Spans<'a>>> for Text<'a> {
    fn from(lines: Vec<Spans<'a>>) -> Text<'a> {
        Text { lines }
    }
}

impl<'a> IntoIterator for Text<'a> {
    type Item = Spans<'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}

impl<'a> Extend<Spans<'a>> for Text<'a> {
    fn extend<T: IntoIterator<Item = Spans<'a>>>(&mut self, iter: T) {
        self.lines.extend(iter);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WrappedText<'a> {
    text: Text<'a>,
    trim: bool,
    width: u16,
    column: u16,
    last_word_end: u16,
    was_whitespace: bool,
    was_linebreak:bool,
}

impl<'a> WrappedText<'a> {
    pub fn new(width: u16) -> Self {
        Self {
            text: Text::default(),
            width,
            trim: true,
            column: 0,
            last_word_end: 0,
            was_whitespace: false,
            was_linebreak:false,
        }
    }

    pub fn trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }

    fn push_span(&mut self, span: Span<'a>) {
        if self.text.lines.is_empty() {
            self.text.lines.push(Spans::default());
        }
        let last_line = self.text.lines.len() - 1;
        self.text.lines[last_line].0.push(span);
    }

    fn push_spans<T>(&mut self, spans: T)
    where
        T: IntoIterator<Item = Span<'a>>,
    {
        let mut iter = spans.into_iter();
        let mut pending_span = iter.next();
        while let Some(mut span) = pending_span.take() {
            let span_position = self.column;
            let mut breakpoint = None;
            // Skip leading whitespaces when trim is enabled
            if self.column == 0 && self.trim {
                span.trim_start();
            }
            for grapheme in UnicodeSegmentation::graphemes(span.content.as_ref(), true) {
                let grapheme_width = grapheme.width() as u16;
                // Ignore grapheme that are larger than the allowed width
                if grapheme_width > self.width {
                    continue;
                }
                if !self.was_linebreak && grapheme =="\n"
                {
                    let width = self.last_word_end.saturating_sub(span_position) as usize;
                    breakpoint = Some(width+1);
                    self.was_linebreak =true;
                    break;
                }
                let is_whitespace = grapheme.chars().all(&char::is_whitespace);
                if  !self.was_whitespace && grapheme != NBSP {
                    self.last_word_end = self.column;
                }
                let next_column = self.column.saturating_add(grapheme_width);
                if next_column > self.width {
                    let width = self.last_word_end.saturating_sub(span_position) as usize;
                    breakpoint = Some(width);
                    break;
                }
                self.column = next_column;
                self.was_whitespace = is_whitespace;
            }
            if let Some(b) = breakpoint {
                pending_span = if b > 0 {
                    let new_span = span.split_at_in_place(b);
                    self.push_span(span);
                    Some(new_span)
                } else {
                    Some(span)
                };
                self.start_new_line();
            } else {
                self.push_span(span);
                pending_span = iter.next();
            }
        }
    }

    fn start_new_line(&mut self) {
        self.column = 0;
        self.last_word_end = 0;
        self.text.lines.push(Spans::default());
    }
}

impl<'a> Extend<Spans<'a>> for WrappedText<'a> {
    fn extend<T: IntoIterator<Item = Spans<'a>>>(&mut self, iter: T) {
        for spans in iter {
            self.start_new_line();
            self.push_spans(spans);
        }
    }
}

impl<'a> From<WrappedText<'a>> for Text<'a> {
    fn from(text: WrappedText<'a>) -> Text<'a> {
        text.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Color,Style};

    #[test]
    fn text_can_be_wrapped() {
        let mut t = WrappedText::new(10);
        t.extend(Text::from(Spans::from(vec![
            Span::raw("This is "),
            Span::styled("a test.", Style::default().fg(Color::Red)),
        ])));
        t.extend(Text::from("It should wrap."));
        let t = Text::from(t);
        let expected = Text::from(vec![
            Spans::from(vec![
                Span::raw("This is "),
                Span::styled("a", Style::default().fg(Color::Red)),
            ]),
            Spans::from(Span::styled("test.", Style::default().fg(Color::Red))),
            Spans::from("It should"),
            Spans::from("wrap."),
        ]);
        assert_eq!(expected, t);
    }

//     #[test]
//     fn text_with_trailing_nbsp_can_be_wrapped() {
//         let mut t = WrappedText::new(10);
//         t.extend(Text::from(Spans::from(vec![
//             Span::raw("Line1"),
//             Span::styled(NBSP, Style::default().add_modifier(Modifier::UNDERLINED)),
//             Span::raw("Line2"),
//         ])));
//         let expected = Text::from(vec![
//             Spans::from(vec![
//                 Span::raw("Line1"),
//                 Span::styled(NBSP, Style::default().add_modifier(Modifier::UNDERLINED)),
//             ]),
//             Spans::from(vec![Span::raw("Line2")]),
//         ]);
//         assert_eq!(expected, Text::from(t));
//     }
 }
