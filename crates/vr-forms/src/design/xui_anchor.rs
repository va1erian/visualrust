//! Mapping the persisted `.vrform` anchor onto xui's layout [`Anchor`].
//!
//! The model keeps its own serde `Anchor`: a `.vrform` is written before any
//! window exists, and the model crate must not depend on a UI toolkit for a
//! value it merely stores. This is the one-way bridge a host uses when it hands
//! a form to xui's `Layout::free`, whose placement pass is anchor-aware.

use xui::Anchor as XuiAnchor;

use crate::model::Anchor;

/// The xui anchor equivalent of a persisted [`Anchor`].
pub fn to_xui(anchor: Anchor) -> XuiAnchor {
    match anchor {
        Anchor::TopLeft => XuiAnchor::TopLeft,
        Anchor::Top => XuiAnchor::Top,
        Anchor::TopRight => XuiAnchor::TopRight,
        Anchor::Left => XuiAnchor::Left,
        Anchor::Center => XuiAnchor::Center,
        Anchor::Right => XuiAnchor::Right,
        Anchor::BottomLeft => XuiAnchor::BottomLeft,
        Anchor::Bottom => XuiAnchor::Bottom,
        Anchor::BottomRight => XuiAnchor::BottomRight,
        Anchor::StretchHorizontal => XuiAnchor::StretchHorizontal,
        Anchor::StretchVertical => XuiAnchor::StretchVertical,
        Anchor::Fill => XuiAnchor::Fill,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_persisted_anchor_maps_to_its_xui_equivalent() {
        let pairs = [
            (Anchor::TopLeft, XuiAnchor::TopLeft),
            (Anchor::Top, XuiAnchor::Top),
            (Anchor::TopRight, XuiAnchor::TopRight),
            (Anchor::Left, XuiAnchor::Left),
            (Anchor::Center, XuiAnchor::Center),
            (Anchor::Right, XuiAnchor::Right),
            (Anchor::BottomLeft, XuiAnchor::BottomLeft),
            (Anchor::Bottom, XuiAnchor::Bottom),
            (Anchor::BottomRight, XuiAnchor::BottomRight),
            (Anchor::StretchHorizontal, XuiAnchor::StretchHorizontal),
            (Anchor::StretchVertical, XuiAnchor::StretchVertical),
            (Anchor::Fill, XuiAnchor::Fill),
        ];
        for (model, expected) in pairs {
            assert_eq!(to_xui(model), expected, "{model:?}");
        }
    }
}
