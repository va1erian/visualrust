//! Value types for the typed wrapper: colours and the small Scintilla enums.
//!
//! The enum discriminants mirror `Scintilla.h`; they are converted with
//! `as i32` at the call boundary.

/// An RGB colour. Scintilla stores colours as `0x00BBGGRR`, so the conversion
/// swaps the channel order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Color {
    /// Builds a colour from its channels.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }

    /// Packs the colour into the `0x00BBGGRR` value Scintilla expects.
    pub const fn to_scintilla(self) -> i32 {
        ((self.b as i32) << 16) | ((self.g as i32) << 8) | (self.r as i32)
    }

    /// Unpacks a `0x00BBGGRR` value.
    pub const fn from_scintilla(value: i32) -> Color {
        Color {
            r: value as u8,
            g: (value >> 8) as u8,
            b: (value >> 16) as u8,
        }
    }
}

/// `STYLE_DEFAULT`, the style Scintilla copies to the palette, from
/// `Scintilla.h`.
pub const STYLE_DEFAULT: u8 = 32;
/// `STYLE_LINENUMBER`, the style a number margin draws its text with, from
/// `Scintilla.h`.
pub const STYLE_LINENUMBER: u8 = 33;

/// The content a margin shows, `SCI_SETMARGINTYPEN`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarginType {
    /// A symbol margin, driven by markers.
    Symbol = 0,
    /// A line-number margin.
    Number = 1,
    /// A margin drawn in the background colour.
    Back = 2,
    /// A margin drawn in the foreground colour.
    Fore = 3,
    /// A margin showing text set with `SCI_MARGINSETTEXT`.
    Text = 4,
    /// A right-aligned text margin.
    RightText = 5,
    /// A margin drawn in an explicit colour.
    Colour = 6,
}

/// A marker symbol, `SCI_MARKERDEFINE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerSymbol {
    /// A circle.
    Circle = 0,
    /// A rounded rectangle.
    RoundRect = 1,
    /// An arrow pointing right.
    Arrow = 2,
    /// A small rectangle.
    SmallRect = 3,
    /// A short arrow.
    ShortArrow = 4,
    /// Nothing.
    Empty = 5,
    /// An arrow pointing down.
    ArrowDown = 6,
    /// A minus sign.
    Minus = 7,
    /// A plus sign.
    Plus = 8,
    /// A vertical line.
    VLine = 9,
    /// A bottom-left corner.
    LCorner = 10,
    /// A top-left corner.
    TCorner = 11,
    /// A boxed plus.
    BoxPlus = 12,
    /// A boxed plus with a connecting line.
    BoxPlusConnected = 13,
    /// A boxed minus.
    BoxMinus = 14,
    /// A boxed minus with a connecting line.
    BoxMinusConnected = 15,
    /// A curved bottom-left corner.
    LCornerCurve = 16,
    /// A curved top-left corner.
    TCornerCurve = 17,
    /// A circled plus.
    CirclePlus = 18,
    /// A circled plus with a connecting line.
    CirclePlusConnected = 19,
    /// A circled minus.
    CircleMinus = 20,
    /// A circled minus with a connecting line.
    CircleMinusConnected = 21,
    /// A solid background.
    Background = 22,
    /// An ellipsis.
    DotDotDot = 23,
    /// Double arrows.
    Arrows = 24,
    /// A full-width rectangle.
    FullRect = 26,
    /// A left rectangle.
    LeftRect = 27,
    /// An underline.
    Underline = 29,
    /// A bookmark.
    Bookmark = 31,
    /// A character marker; the code point follows in `SCI_MARKERSETCHAR`.
    Character = 10000,
}

/// An indicator's visual style, `SCI_INDICSETSTYLE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndicatorStyle {
    /// A straight underline.
    Plain = 0,
    /// A squiggly underline.
    Squiggle = 1,
    /// A dotted rectangle.
    Tt = 2,
    /// A diagonal hatch.
    Diagonal = 3,
    /// A strike-through line.
    Strike = 4,
    /// Hidden text.
    Hidden = 5,
    /// A box outline.
    Box = 6,
    /// A rounded box outline.
    RoundBox = 7,
    /// A straight box outline.
    StraightBox = 8,
    /// A dashed underline.
    Dash = 9,
    /// A dotted underline.
    Dots = 10,
    /// A low squiggly underline.
    SquiggleLow = 11,
    /// A dotted box.
    DotBox = 12,
    /// A squiggly pixmap underline.
    SquigglePixmap = 13,
    /// A thick composition underline.
    CompositionThick = 14,
    /// A thin composition underline.
    CompositionThin = 15,
    /// A full box.
    FullBox = 16,
    /// Text in the foreground colour.
    TextFore = 17,
    /// A point indicator.
    Point = 18,
}

/// How annotations are shown, `SCI_ANNOTATIONSETVISIBLE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationVisible {
    /// Annotations are hidden.
    Hidden = 0,
    /// Annotations share the line.
    Standard = 1,
    /// Annotations are boxed.
    Boxed = 2,
    /// Annotations are boxed and indented.
    Indented = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_colours_as_bgr() {
        assert_eq!(Color::rgb(0x11, 0x22, 0x33).to_scintilla(), 0x0033_2211);
        assert_eq!(
            Color::from_scintilla(0x0033_2211),
            Color::rgb(0x11, 0x22, 0x33)
        );
    }

    #[test]
    fn enum_codes_match_the_header() {
        assert_eq!(MarginType::Number as i32, 1);
        assert_eq!(MarkerSymbol::Bookmark as i32, 31);
        assert_eq!(IndicatorStyle::Squiggle as i32, 1);
        assert_eq!(AnnotationVisible::Boxed as i32, 2);
    }
}
