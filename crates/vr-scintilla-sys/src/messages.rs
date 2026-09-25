//! `SCI_*` message constants, copied from `vendor/scintilla/include/Scintilla.h`.
//!
//! Only the messages a basic container-lexer editor needs are mirrored here;
//! add more from the header as the wrapper grows, keeping this file under the
//! 500-line limit by splitting it if necessary.
#![forbid(unsafe_code)]

// Document text and length.
pub const SCI_ADDTEXT: u32 = 2001;
pub const SCI_ADDSTYLEDTEXT: u32 = 2002;
pub const SCI_INSERTTEXT: u32 = 2003;
pub const SCI_CLEARALL: u32 = 2004;
pub const SCI_GETLENGTH: u32 = 2006;
pub const SCI_GETCHARAT: u32 = 2007;
pub const SCI_GETCURRENTPOS: u32 = 2008;
pub const SCI_GETANCHOR: u32 = 2009;
pub const SCI_GETSTYLEAT: u32 = 2010;
pub const SCI_SETTEXT: u32 = 2181;
pub const SCI_GETTEXT: u32 = 2182;
pub const SCI_GETTEXTLENGTH: u32 = 2183;
pub const SCI_APPENDTEXT: u32 = 2282;

// Selection and caret.
pub const SCI_SETCURRENTPOS: u32 = 2141;
pub const SCI_SETSELECTIONSTART: u32 = 2142;
pub const SCI_GETSELECTIONSTART: u32 = 2143;
pub const SCI_SETSELECTIONEND: u32 = 2144;
pub const SCI_GETSELECTIONEND: u32 = 2145;
pub const SCI_SETSEL: u32 = 2160;

// Lines.
pub const SCI_GOTOLINE: u32 = 2024;
pub const SCI_GOTOPOS: u32 = 2025;
pub const SCI_GETLINEENDPOSITION: u32 = 2136;
pub const SCI_GETLINECOUNT: u32 = 2154;
pub const SCI_LINEFROMPOSITION: u32 = 2166;
pub const SCI_POSITIONFROMLINE: u32 = 2167;
pub const SCI_LINELENGTH: u32 = 2350;

// Undo, dirty state and read-only flag.
pub const SCI_SETUNDOCOLLECTION: u32 = 2012;
pub const SCI_SETSAVEPOINT: u32 = 2014;
pub const SCI_GETUNDOCOLLECTION: u32 = 2019;
pub const SCI_GETREADONLY: u32 = 2140;
pub const SCI_GETMODIFY: u32 = 2159;
pub const SCI_SETREADONLY: u32 = 2171;
pub const SCI_EMPTYUNDOBUFFER: u32 = 2175;

// Container lexer and styling.
pub const SCI_SETCODEPAGE: u32 = 2037;
pub const SCI_GETENDSTYLED: u32 = 2028;
pub const SCI_STARTSTYLING: u32 = 2032;
pub const SCI_SETSTYLING: u32 = 2033;
pub const SCI_STYLECLEARALL: u32 = 2050;
pub const SCI_STYLESETFORE: u32 = 2051;
pub const SCI_STYLESETBACK: u32 = 2052;
pub const SCI_STYLESETBOLD: u32 = 2053;
pub const SCI_STYLESETITALIC: u32 = 2054;
pub const SCI_STYLESETSIZE: u32 = 2055;
pub const SCI_STYLESETFONT: u32 = 2056;
pub const SCI_SETSTYLINGEX: u32 = 2073;
pub const SCI_STYLEGETFORE: u32 = 2481;
pub const SCI_STYLEGETBACK: u32 = 2482;
pub const SCI_STYLEGETBOLD: u32 = 2483;
pub const SCI_STYLEGETITALIC: u32 = 2484;
pub const SCI_STYLEGETSIZE: u32 = 2485;
pub const SCI_STYLEGETFONT: u32 = 2486;
pub const SCI_GETLEXER: u32 = 4002;
pub const SCI_SETILEXER: u32 = 4033;

// Layout and display options.
pub const SCI_SETBUFFEREDDRAW: u32 = 2035;
pub const SCI_SETMARGINTYPEN: u32 = 2240;
pub const SCI_SETMARGINWIDTHN: u32 = 2242;
pub const SCI_SETMARGINMASKN: u32 = 2244;
pub const SCI_SETWRAPMODE: u32 = 2268;
pub const SCI_GETWRAPMODE: u32 = 2269;
pub const SCI_SETSCROLLWIDTH: u32 = 2274;
pub const SCI_SETTWOPHASEDRAW: u32 = 2284;
pub const SCI_SETMODEVENTMASK: u32 = 2359;
pub const SCI_GETMODEVENTMASK: u32 = 2378;
pub const SCI_SETSCROLLWIDTHTRACKING: u32 = 2516;
pub const SCI_SETFIRSTVISIBLELINE: u32 = 2613;

// Tabs and properties.
pub const SCI_SETTABWIDTH: u32 = 2036;
pub const SCI_GETTABWIDTH: u32 = 2121;
pub const SCI_SETUSETABS: u32 = 2124;
pub const SCI_GETUSETABS: u32 = 2125;
pub const SCI_SETPROPERTY: u32 = 4004;
pub const SCI_SETKEYWORDS: u32 = 4005;
