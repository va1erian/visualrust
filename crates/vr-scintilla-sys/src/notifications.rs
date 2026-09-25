//! `SCN_*` / `SCEN_*` notification codes, copied from
//! `vendor/scintilla/include/Scintilla.h`.
//!
//! These arrive in the high word of `WM_NOTIFY`'s `lParam` as
//! `SCNotification::nmhdr.code`.
#![forbid(unsafe_code)]

pub const SCEN_CHANGE: u32 = 768;
pub const SCEN_SETFOCUS: u32 = 512;
pub const SCEN_KILLFOCUS: u32 = 256;

pub const SCN_STYLENEEDED: u32 = 2000;
pub const SCN_CHARADDED: u32 = 2001;
pub const SCN_SAVEPOINTREACHED: u32 = 2002;
pub const SCN_SAVEPOINTLEFT: u32 = 2003;
pub const SCN_MODIFYATTEMPTRO: u32 = 2004;
pub const SCN_KEY: u32 = 2005;
pub const SCN_DOUBLECLICK: u32 = 2006;
pub const SCN_UPDATEUI: u32 = 2007;
pub const SCN_MODIFIED: u32 = 2008;
pub const SCN_MACRORECORD: u32 = 2009;
pub const SCN_MARGINCLICK: u32 = 2010;
pub const SCN_NEEDSHOWN: u32 = 2011;
pub const SCN_PAINTED: u32 = 2013;
pub const SCN_USERLISTSELECTION: u32 = 2014;
pub const SCN_URIDROPPED: u32 = 2015;
pub const SCN_DWELLSTART: u32 = 2016;
pub const SCN_DWELLEND: u32 = 2017;
pub const SCN_ZOOM: u32 = 2018;
pub const SCN_HOTSPOTCLICK: u32 = 2019;
pub const SCN_HOTSPOTDOUBLECLICK: u32 = 2020;
pub const SCN_CALLTIPCLICK: u32 = 2021;
pub const SCN_AUTOCSELECTION: u32 = 2022;
pub const SCN_INDICATORCLICK: u32 = 2023;
pub const SCN_INDICATORRELEASE: u32 = 2024;
pub const SCN_AUTOCCANCELLED: u32 = 2025;
pub const SCN_AUTOCCHARDELETED: u32 = 2026;
pub const SCN_HOTSPOTRELEASECLICK: u32 = 2027;
pub const SCN_FOCUSIN: u32 = 2028;
pub const SCN_FOCUSOUT: u32 = 2029;
pub const SCN_AUTOCCOMPLETED: u32 = 2030;
pub const SCN_MARGINRIGHTCLICK: u32 = 2031;
pub const SCN_AUTOCSELECTIONCHANGE: u32 = 2032;
