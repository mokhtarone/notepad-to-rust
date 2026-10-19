//! Constants, resource IDs and enums — port of notepad_res.h + notepad.h.
//! Values match the ReactOS master headers (the binary used STRING ids +0x10,
//! but since we build every resource at runtime, master ids are used here).

#![allow(dead_code)]

// ---------------------------------------------------------------- resources
pub const MAIN_MENU: u16 = 0x201;
pub const DIALOG_PAGESETUP: u16 = 0x202;
pub const ID_ACCEL: u16 = 0x203;
pub const DIALOG_ENCODING: u16 = 0x204;
pub const ID_ENCODING: u16 = 0x205;
pub const ID_EOLN: u16 = 0x206;
pub const DIALOG_GOTO: u16 = 0x207;
pub const ID_LINENUMBER: u16 = 0x208;
pub const IDI_NPICON: u16 = 0x209;
pub const DIALOG_PRINTING: u16 = 0x20B;
pub const IDC_PRINTING_STATUS: u16 = 0x20C;
pub const IDC_PRINTING_FILENAME: u16 = 0x20D;
pub const IDC_PRINTING_PAGE: u16 = 0x20E;

// ---------------------------------------------------------------- commands
pub const CMD_NEW: u32 = 0x100;
pub const CMD_NEW_WINDOW: u32 = 0x101;
pub const CMD_OPEN: u32 = 0x102;
pub const CMD_SAVE: u32 = 0x103;
pub const CMD_SAVE_AS: u32 = 0x104;
pub const CMD_PRINT: u32 = 0x105;
pub const CMD_PAGE_SETUP: u32 = 0x106;
pub const CMD_EXIT: u32 = 0x108;

pub const CMD_UNDO: u32 = 0x110;
pub const CMD_CUT: u32 = 0x111;
pub const CMD_COPY: u32 = 0x112;
pub const CMD_PASTE: u32 = 0x113;
pub const CMD_DELETE: u32 = 0x114;
pub const CMD_SELECT_ALL: u32 = 0x116;
pub const CMD_TIME_DATE: u32 = 0x117;

pub const CMD_SEARCH: u32 = 0x120;
pub const CMD_SEARCH_NEXT: u32 = 0x121;
pub const CMD_REPLACE: u32 = 0x122;
pub const CMD_GOTO: u32 = 0x123;
pub const CMD_SEARCH_PREV: u32 = 0x124;

pub const CMD_WRAP: u32 = 0x119;
pub const CMD_FONT: u32 = 0x140;

pub const CMD_STATUSBAR: u32 = 0x150;
pub const CMD_STATUSBAR_WND_ID: u32 = 0x151;

pub const CMD_HELP_CONTENTS: u32 = 0x130;
pub const CMD_HELP_ABOUT_NOTEPAD: u32 = 0x135;

// ---------------------------------------------------------------- strings
pub const STRING_PAGESETUP_HEADERVALUE: u32 = 0x160;
pub const STRING_PAGESETUP_FOOTERVALUE: u32 = 0x161;

pub const STRING_NOTEPAD: u32 = 0x170;
pub const STRING_ERROR: u32 = 0x171;
pub const STRING_WARNING: u32 = 0x172;
pub const STRING_INFO: u32 = 0x173;
pub const STRING_UNTITLED: u32 = 0x174;
pub const STRING_ALL_FILES: u32 = 0x175;
pub const STRING_TEXT_FILES_TXT: u32 = 0x176;
pub const STRING_TOOLARGE: u32 = 0x177;
pub const STRING_NOTEXT: u32 = 0x178;
pub const STRING_DOESNOTEXIST: u32 = 0x179;
pub const STRING_NOTSAVED: u32 = 0x17A;

pub const STRING_NOTFOUND: u32 = 0x17B;
pub const STRING_OUT_OF_MEMORY: u32 = 0x17C;
pub const STRING_CANNOTFIND: u32 = 0x17D;

pub const STRING_ANSI: u32 = 0x180;
pub const STRING_UNICODE: u32 = 0x181;
pub const STRING_UNICODE_BE: u32 = 0x182;
pub const STRING_UTF8: u32 = 0x183;
pub const STRING_UTF8_BOM: u32 = 0x184;

pub const STRING_CRLF: u32 = 0x185;
pub const STRING_LF: u32 = 0x186;
pub const STRING_CR: u32 = 0x187;

pub const STRING_LINE_COLUMN: u32 = 0x188;
pub const STRING_PRINTERROR: u32 = 0x189;
pub const STRING_DEFAULTFONT: u32 = 0x18A;
pub const STRING_LINE_NUMBER_OUT_OF_RANGE: u32 = 0x18B;
pub const STRING_NOWPRINTING: u32 = 0x18C;
pub const STRING_PRINTCANCELING: u32 = 0x18D;
pub const STRING_PRINTCOMPLETE: u32 = 0x18E;
pub const STRING_PRINTCANCELED: u32 = 0x18F;
pub const STRING_PRINTFAILED: u32 = 0x190;

pub const STRING_TEXT_DOCUMENT: u32 = 0x200;
pub const STRING_NOTEPAD_AUTHORS: u32 = 0x300;

// ---------------------------------------------------------------- encodings
/// Values are indexes of the items in the Encoding combobox.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encoding {
    Auto = -1,
    Ansi = 0,
    Utf16Le = 1,
    Utf16Be = 2,
    Utf8 = 3,
    Utf8Bom = 4,
}
pub const ENCODING_DEFAULT: Encoding = Encoding::Utf8;

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Eoln {
    CrLf = 0, // "\r\n"
    Lf = 1,   // "\n"
    Cr = 2,   // "\r"
}

impl Encoding {
    pub fn from_i32(v: i32) -> Encoding {
        match v {
            0 => Encoding::Ansi,
            1 => Encoding::Utf16Le,
            2 => Encoding::Utf16Be,
            4 => Encoding::Utf8Bom,
            _ => Encoding::Utf8,
        }
    }
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

impl Eoln {
    pub fn from_u32(v: u32) -> Eoln {
        match v {
            1 => Eoln::Lf,
            2 => Eoln::Cr,
            _ => Eoln::CrLf,
        }
    }
}

// ---------------------------------------------------------------- edit styles
pub const WS_CHILD: u32 = 0x4000_0000;
pub const WS_VISIBLE: u32 = 0x1000_0000;
pub const WS_VSCROLL: u32 = 0x0020_0000;
pub const WS_HSCROLL: u32 = 0x0010_0000;
pub const ES_AUTOVSCROLL: u32 = 0x0040;
pub const ES_MULTILINE: u32 = 0x0004;
pub const ES_NOHIDESEL: u32 = 0x0100;
pub const ES_AUTOHSCROLL: u32 = 0x0080;

pub const EDIT_STYLE_WRAP: u32 =
    WS_CHILD | WS_VSCROLL | ES_AUTOVSCROLL | ES_MULTILINE | ES_NOHIDESEL;
pub const EDIT_STYLE: u32 = EDIT_STYLE_WRAP | WS_HSCROLL | ES_AUTOHSCROLL;
pub const EDIT_CLASS: &str = "EDIT";

pub const MAX_STRING_LEN: usize = 255;
pub const MAX_PATH_US: usize = 260;

// ------------------------------------------------- comdlg32 template ids (dlgs.h)
pub const DLG_PSH3: u16 = 0x0402; // "Printer..."
pub const DLG_RAD1: u16 = 0x0420; // Portrait
pub const DLG_RAD2: u16 = 0x0421; // Landscape
pub const DLG_GRP1: u16 = 0x0430; // Orientation
pub const DLG_GRP2: u16 = 0x0431; // Paper
pub const DLG_GRP4: u16 = 0x0433; // Borders/Margins
pub const DLG_RCT1: u16 = 0x0438; // preview white
pub const DLG_RCT2: u16 = 0x0439; // preview gray right
pub const DLG_RCT3: u16 = 0x043A; // preview gray bottom
pub const DLG_STC2: u16 = 0x0441; // "&Size:"
pub const DLG_STC3: u16 = 0x0442; // "&Tray:"
pub const DLG_STC15: u16 = 0x044E; // "Left:"
pub const DLG_STC16: u16 = 0x044F; // "Right:"
pub const DLG_STC17: u16 = 0x0450; // "Top:"
pub const DLG_STC18: u16 = 0x0451; // "Bottom:"
pub const DLG_CMB2: u16 = 0x0471; // paper size combo
pub const DLG_CMB3: u16 = 0x0472; // paper tray combo
pub const DLG_EDT4: u16 = 0x0483; // left margin
pub const DLG_EDT5: u16 = 0x0484; // top margin
pub const DLG_EDT6: u16 = 0x0485; // right margin
pub const DLG_EDT7: u16 = 0x0486; // bottom margin

pub const IDHELP: u16 = 9;

/// WM_USER + 100 — private message used by the Printing dialog.
pub const WM_USER: u32 = 0x0400;
pub const PRINTING_MESSAGE: u32 = WM_USER + 100;



