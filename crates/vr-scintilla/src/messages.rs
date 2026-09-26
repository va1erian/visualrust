//! `SCI_*` values the wrapper needs that are not in `vr-scintilla-sys`'s
//! curated set, copied from `vendor/scintilla/include/Scintilla.h`.
//!
//! The remainder are re-exported so call sites and tests can reach every
//! constant through one module.

pub use vr_scintilla_sys::messages::*;

// Margins and markers.
pub const SCI_MARKERDEFINE: u32 = 2040;
pub const SCI_MARKERADD: u32 = 2043;
pub const SCI_MARKERDELETE: u32 = 2044;
pub const SCI_MARGINSETTEXT: u32 = 2530;

// Indicators.
pub const SCI_INDICSETSTYLE: u32 = 2080;
pub const SCI_INDICSETFORE: u32 = 2082;
pub const SCI_INDICSETALPHA: u32 = 2523;
pub const SCI_SETINDICATORCURRENT: u32 = 2500;
pub const SCI_SETINDICATORVALUE: u32 = 2502;
pub const SCI_INDICATORFILLRANGE: u32 = 2504;
pub const SCI_INDICATORCLEARRANGE: u32 = 2505;

// Annotations.
pub const SCI_ANNOTATIONSETTEXT: u32 = 2540;
pub const SCI_ANNOTATIONSETVISIBLE: u32 = 2548;

// Autocompletion and call tips.
pub const SCI_AUTOCSHOW: u32 = 2100;
pub const SCI_AUTOCCANCEL: u32 = 2101;
pub const SCI_CALLTIPSHOW: u32 = 2200;
pub const SCI_CALLTIPCANCEL: u32 = 2201;

// Brace matching.
pub const SCI_BRACEMATCH: u32 = 2353;

// Margins, selection and caret colours. A number margin draws its text in
// `STYLE_LINENUMBER`; the margin background is set separately with
// `SCI_SETMARGINBACKN`.
pub const SCI_SETMARGINBACKN: u32 = 2250;
pub const SCI_SETSELFORE: u32 = 2067;
pub const SCI_SETSELBACK: u32 = 2068;
pub const SCI_SETCARETFORE: u32 = 2069;
