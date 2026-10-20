//! Runtime resource construction — main menu, accelerator table, dialog
//! templates and the application icon (no .rc compilation).
//! Everything is built in memory from the decoded original resources.

#![allow(non_upper_case_globals)]

use crate::consts::*;
use crate::strings::wide;
use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateAcceleratorTableW, CreateIconFromResourceEx, CreateMenu, CreatePopupMenu,
    ACCEL, FCONTROL, FSHIFT, FVIRTKEY, HACCEL, HMENU, LR_DEFAULTCOLOR, MENU_ITEM_FLAGS, MF_POPUP,
    HICON, MF_SEPARATOR, MF_STRING,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_F3, VK_F5};

const NOTEPAD_ICO: &[u8] = include_bytes!("../assets/notepad.ico");

fn last_error() -> windows::core::Error {
    // SAFETY: GetLastError only reads the thread's last error code.
    unsafe {
        windows::core::Error::from_hresult(windows::core::HRESULT::from_win32(
            windows::Win32::Foundation::GetLastError().0,
        ))
    }
}

// ---------------------------------------------------------------- styles
const WS_CAPTION: u32 = 0x00C0_0000;
const WS_SYSMENU: u32 = 0x0008_0000;
const WS_TABSTOP: u32 = 0x0001_0000;
const WS_GROUP: u32 = 0x0002_0000;
const WS_BORDER: u32 = 0x0080_0000;
const WS_CLIPSIBLINGS: u32 = 0x0400_0000;
const DS_SETFONT: u32 = 0x0040;
const DS_FIXEDSYS: u32 = 0x0008;
const DS_MODALFRAME: u32 = 0x0080;
const DS_CONTROL: u32 = 0x0400;
const DS_CONTEXTHELP: u32 = 0x2000;
const DS_SHELLFONT: u32 = DS_SETFONT | DS_FIXEDSYS;

const BS_PUSHBUTTON: u32 = 0x0000_0000;
const BS_DEFPUSHBUTTON: u32 = 0x0000_0001;
const BS_GROUPBOX: u32 = 0x0000_0004;
const BS_AUTORADIOBUTTON: u32 = 0x0000_0009;
const SS_LEFT: u32 = 0x0000_0000;
const SS_CENTER: u32 = 0x0000_0001;
const SS_GRAYRECT: u32 = 0x0000_0005;
const SS_WHITERECT: u32 = 0x0000_0006;
const CBS_SIMPLE: u32 = 0x0000_0001;
const CBS_DROPDOWN: u32 = 0x0000_0002;
const CBS_DROPDOWNLIST: u32 = 0x0000_0003;
const CBS_SORT: u32 = 0x0000_0100;
const ES_NUMBER: u32 = 0x0000_2000;
const ES_LEFT: u32 = 0x0000_0000;

const IDOK_ID: u16 = 1;
const IDCANCEL_ID: u16 = 2;

#[inline]
fn child(style: u32) -> u32 {
    style | WS_CHILD | WS_VISIBLE
}

// ---------------------------------------------------------------- icon
unsafe fn icon_from_ico(cx: i32, cy: i32) -> windows::core::Result<HICON> {
    if NOTEPAD_ICO.len() < 6 {
        return Err(last_error());
    }
    let count = u16::from_le_bytes([NOTEPAD_ICO[4], NOTEPAD_ICO[5]]) as usize;
    let mut best: Option<(usize, usize, u16)> = None; // (offset, size, bpp)
    for i in 0..count {
        let e = 6 + i * 16;
        if e + 16 > NOTEPAD_ICO.len() {
            break;
        }
        let w = NOTEPAD_ICO[e];
        let h = NOTEPAD_ICO[e + 1];
        let w = if w == 0 { 256 } else { w as i32 };
        let h = if h == 0 { 256 } else { h as i32 };
        let bpp = u16::from_le_bytes([NOTEPAD_ICO[e + 6], NOTEPAD_ICO[e + 7]]);
        let size = u32::from_le_bytes([
            NOTEPAD_ICO[e + 8],
            NOTEPAD_ICO[e + 9],
            NOTEPAD_ICO[e + 10],
            NOTEPAD_ICO[e + 11],
        ]) as usize;
        let offset = u32::from_le_bytes([
            NOTEPAD_ICO[e + 12],
            NOTEPAD_ICO[e + 13],
            NOTEPAD_ICO[e + 14],
            NOTEPAD_ICO[e + 15],
        ]) as usize;
        if w == cx && h == cy && offset + size <= NOTEPAD_ICO.len() {
            if best.map(|b| bpp > b.2).unwrap_or(true) {
                best = Some((offset, size, bpp));
            }
        }
    }
    let (offset, size, _bpp) = best.ok_or_else(last_error)?;
    CreateIconFromResourceEx(
        &NOTEPAD_ICO[offset..offset + size],
        true,
        0x0003_0000,
        cx,
        cy,
        LR_DEFAULTCOLOR,
    )
}

/// LoadIcon(IDI_NPICON) equivalent — the 32x32 icon.
pub unsafe fn load_icon() -> windows::core::Result<HICON> {
    icon_from_ico(32, 32).or_else(|_| icon_from_ico(48, 48))
}

/// The small (16x16) icon, for WM_SETICON ICON_SMALL.
pub unsafe fn load_small_icon() -> windows::core::Result<HICON> {
    icon_from_ico(16, 16).or_else(|_| icon_from_ico(32, 32))
}

// ---------------------------------------------------------------- menu
unsafe fn add_item(menu: HMENU, flags: MENU_ITEM_FLAGS, id: usize, text: &str) {
    let _ = AppendMenuW(menu, flags, id, PCWSTR(wide(text).as_ptr()));
}

/// Build the main menu (port of MAIN_MENU from en-US.rc).
pub unsafe fn load_main_menu() -> windows::core::Result<HMENU> {
    let main = CreateMenu()?;

    // --- &File ---
    let file = CreatePopupMenu()?;
    add_item(file, MF_STRING, CMD_NEW as usize, "&New\tCtrl+N");
    add_item(file, MF_STRING, CMD_NEW_WINDOW as usize, "New &Window\tCtrl+Shift+N");
    add_item(file, MF_STRING, CMD_OPEN as usize, "&Open...\tCtrl+O");
    add_item(file, MF_STRING, CMD_SAVE as usize, "&Save\tCtrl+S");
    add_item(file, MF_STRING, CMD_SAVE_AS as usize, "Save &As...");
    add_item(file, MF_SEPARATOR, 0, "");
    add_item(file, MF_STRING, CMD_PAGE_SETUP as usize, "Page Set&up...");
    add_item(file, MF_STRING, CMD_PRINT as usize, "&Print...\tCtrl+P");
    add_item(file, MF_SEPARATOR, 0, "");
    add_item(file, MF_STRING, CMD_EXIT as usize, "E&xit");
    add_item(main, MF_POPUP, file.0 as usize, "&File");

    // --- &Edit ---
    let edit = CreatePopupMenu()?;
    add_item(edit, MF_STRING, CMD_UNDO as usize, "&Undo\tCtrl+Z");
    add_item(edit, MF_SEPARATOR, 0, "");
    add_item(edit, MF_STRING, CMD_CUT as usize, "Cu&t\tCtrl+X");
    add_item(edit, MF_STRING, CMD_COPY as usize, "&Copy\tCtrl+C");
    add_item(edit, MF_STRING, CMD_PASTE as usize, "&Paste\tCtrl+V");
    add_item(edit, MF_STRING, CMD_DELETE as usize, "De&lete\tDel");
    add_item(edit, MF_SEPARATOR, 0, "");
    add_item(edit, MF_STRING, CMD_SEARCH as usize, "&Find...\tCtrl+F");
    add_item(edit, MF_STRING, CMD_SEARCH_NEXT as usize, "Find &Next\tF3");
    add_item(edit, MF_STRING, CMD_REPLACE as usize, "&Replace...\tCtrl+H");
    add_item(edit, MF_STRING, CMD_GOTO as usize, "&Go To...\tCtrl+G");
    add_item(edit, MF_SEPARATOR, 0, "");
    add_item(edit, MF_STRING, CMD_SELECT_ALL as usize, "Select &All\tCtrl+A");
    add_item(edit, MF_STRING, CMD_TIME_DATE as usize, "Time/&Date\tF5");
    add_item(main, MF_POPUP, edit.0 as usize, "&Edit");

    // --- F&ormat ---
    let format = CreatePopupMenu()?;
    add_item(format, MF_STRING, CMD_WRAP as usize, "&Word Wrap");
    add_item(format, MF_STRING, CMD_FONT as usize, "&Font...");
    add_item(main, MF_POPUP, format.0 as usize, "F&ormat");

    // --- &View ---
    let view = CreatePopupMenu()?;
    add_item(view, MF_STRING, CMD_STATUSBAR as usize, "&Status Bar");
    add_item(main, MF_POPUP, view.0 as usize, "&View");

    // --- &Help ---
    let help = CreatePopupMenu()?;
    add_item(help, MF_STRING, CMD_HELP_CONTENTS as usize, "View &Help");
    add_item(help, MF_SEPARATOR, 0, "");
    add_item(help, MF_STRING, CMD_HELP_ABOUT_NOTEPAD as usize, "&About Notepad");
    add_item(main, MF_POPUP, help.0 as usize, "&Help");

    Ok(main)
}

// ---------------------------------------------------------------- accelerators
/// Build the accelerator table (port of ID_ACCEL from en-US.rc,
/// matching the decoded binary table).
pub unsafe fn load_accelerators() -> windows::core::Result<HACCEL> {
    let accels = [
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'A' as u16, cmd: CMD_SELECT_ALL as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'C' as u16, cmd: CMD_COPY as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'F' as u16, cmd: CMD_SEARCH as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'G' as u16, cmd: CMD_GOTO as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'H' as u16, cmd: CMD_REPLACE as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'N' as u16, cmd: CMD_NEW as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL | FSHIFT, key: 'N' as u16, cmd: CMD_NEW_WINDOW as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'O' as u16, cmd: CMD_OPEN as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'P' as u16, cmd: CMD_PRINT as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'S' as u16, cmd: CMD_SAVE as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'V' as u16, cmd: CMD_PASTE as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'X' as u16, cmd: CMD_CUT as u16 },
        ACCEL { fVirt: FVIRTKEY | FCONTROL, key: 'Z' as u16, cmd: CMD_UNDO as u16 },
        ACCEL { fVirt: FVIRTKEY, key: VK_F3.0 as u16, cmd: CMD_SEARCH_NEXT as u16 },
        ACCEL { fVirt: FVIRTKEY | FSHIFT, key: VK_F3.0 as u16, cmd: CMD_SEARCH_PREV as u16 },
        ACCEL { fVirt: FVIRTKEY, key: VK_F5.0 as u16, cmd: CMD_TIME_DATE as u16 },
    ];
    CreateAcceleratorTableW(&accels)
}

// ------------------------------------------------- dialog template builder
fn push_wstr(buf: &mut Vec<u8>, s: &str) {
    for u in s.encode_utf16() {
        buf.extend_from_slice(&u.to_le_bytes());
    }
    buf.extend_from_slice(&0u16.to_le_bytes());
}

/// Builder for classic DLGTEMPLATE memory images.
struct DlgTemplate {
    buf: Vec<u8>,
}

impl DlgTemplate {
    #[allow(clippy::too_many_arguments)]
    fn new(
        style: u32,
        ex_style: u32,
        cdit: u16,
        x: i16,
        y: i16,
        cx: i16,
        cy: i16,
        caption: &str,
        font: Option<(u16, &str)>,
    ) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&style.to_le_bytes());
        buf.extend_from_slice(&ex_style.to_le_bytes());
        buf.extend_from_slice(&cdit.to_le_bytes());
        buf.extend_from_slice(&(x as u16).to_le_bytes());
        buf.extend_from_slice(&(y as u16).to_le_bytes());
        buf.extend_from_slice(&(cx as u16).to_le_bytes());
        buf.extend_from_slice(&(cy as u16).to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes()); // no menu
        buf.extend_from_slice(&0u16.to_le_bytes()); // default dialog class
        push_wstr(&mut buf, caption); // title
        if let Some((pt, name)) = font {
            buf.extend_from_slice(&pt.to_le_bytes());
            push_wstr(&mut buf, name);
        }
        DlgTemplate { buf }
    }

    fn align4(&mut self) {
        while self.buf.len() % 4 != 0 {
            self.buf.push(0);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        style: u32,
        ex: u32,
        x: i16,
        y: i16,
        cx: i16,
        cy: i16,
        id: u16,
        class: &str,
        title: &str,
    ) {
        self.align4();
        self.buf.extend_from_slice(&style.to_le_bytes());
        self.buf.extend_from_slice(&ex.to_le_bytes());
        self.buf.extend_from_slice(&(x as u16).to_le_bytes());
        self.buf.extend_from_slice(&(y as u16).to_le_bytes());
        self.buf.extend_from_slice(&(cx as u16).to_le_bytes());
        self.buf.extend_from_slice(&(cy as u16).to_le_bytes());
        self.buf.extend_from_slice(&id.to_le_bytes());
        push_wstr(&mut self.buf, class); // class name
        push_wstr(&mut self.buf, title); // title / text
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // no extra data
    }

    fn finish(mut self) -> Vec<u8> {
        self.align4();
        self.buf
    }
}

// ------------------------------------------------- dialog templates

/// DIALOG_GOTO (0x207) — "Goto line" dialog.
pub fn goto_dialog_template() -> Vec<u8> {
    let mut t = DlgTemplate::new(
        DS_SHELLFONT | DS_MODALFRAME | WS_CAPTION | WS_SYSMENU,
        0,
        4,
        0,
        0,
        165,
        50,
        "Goto line",
        Some((8, "MS Shell Dlg")),
    );
    t.add(child(SS_LEFT | WS_GROUP), 0, 5, 12, 41, 12, 0x155, "Static", "Line number:");
    t.add(
        child(ES_LEFT | WS_BORDER | WS_TABSTOP | ES_NUMBER),
        0, 54, 10, 106, 12, ID_LINENUMBER, "Edit", "",
    );
    t.add(child(BS_DEFPUSHBUTTON | WS_TABSTOP), 0, 75, 30, 40, 15, IDOK_ID, "Button", "OK");
    t.add(child(BS_PUSHBUTTON | WS_TABSTOP), 0, 120, 30, 40, 15, IDCANCEL_ID, "Button", "Cancel");
    t.finish()
}

/// DIALOG_ENCODING (0x204) — Encoding/EOLN child of the Save-As dialog.
pub fn encoding_dialog_template() -> Vec<u8> {
    let mut t = DlgTemplate::new(
        DS_SHELLFONT | DS_CONTROL | WS_CHILD | WS_CLIPSIBLINGS | WS_CAPTION | WS_SYSMENU,
        0,
        4,
        0,
        0,
        256,
        44,
        "Encoding",
        Some((8, "MS Shell Dlg")),
    );
    t.add(
        child(CBS_DROPDOWNLIST | CBS_SORT | WS_VSCROLL | WS_TABSTOP),
        0, 124, 0, 125, 80, ID_ENCODING, "ComboBox", "",
    );
    t.add(child(SS_LEFT | WS_GROUP), 0, 65, 2, 41, 12, 0x155, "Static", "Encoding:");
    t.add(
        child(CBS_DROPDOWNLIST | WS_VSCROLL | WS_TABSTOP),
        0, 124, 18, 125, 80, ID_EOLN, "ComboBox", "",
    );
    t.add(child(SS_LEFT | WS_GROUP), 0, 65, 20, 41, 12, 0x156, "Static", "Endlines:");
    t.finish()
}

/// DIALOG_PRINTING (0x20B) — printing progress dialog.
pub fn printing_dialog_template() -> Vec<u8> {
    let mut t = DlgTemplate::new(
        DS_SHELLFONT | DS_MODALFRAME | WS_CAPTION | WS_SYSMENU,
        0,
        4,
        0,
        0,
        160,
        100,
        "Now printing",
        Some((8, "MS Shell Dlg")),
    );
    t.add(child(SS_CENTER | WS_GROUP), 0, 5, 10, 150, 15, IDC_PRINTING_STATUS, "Static", "Print job is starting...");
    t.add(child(SS_CENTER | WS_GROUP), 0, 5, 35, 150, 15, IDC_PRINTING_FILENAME, "Static", "(Filename)");
    t.add(child(SS_CENTER | WS_GROUP), 0, 5, 55, 150, 15, IDC_PRINTING_PAGE, "Static", "Page %u");
    t.add(child(BS_PUSHBUTTON | WS_TABSTOP), 0, 50, 75, 60, 20, IDCANCEL_ID, "Button", "Cancel");
    t.finish()
}

/// DIALOG_PAGESETUP (0x202) — extends the comdlg32 PageSetup template
/// with dlgs.h control IDs (used via PSD_ENABLEPAGESETUPTEMPLATE).
/// Not wired yet: comdlg32 resolves PSD_ENABLEPAGESETUPTEMPLATE through
/// FindResourceW on hInstance, so a memory image cannot be passed directly;
/// this binary ships no .res. Kept for a future resource-based wiring.
#[allow(dead_code)]
pub fn pagesetup_dialog_template() -> Vec<u8> {
    let mut t = DlgTemplate::new(
        DS_SHELLFONT | DS_MODALFRAME | WS_CAPTION | WS_SYSMENU | DS_CONTEXTHELP,
        0,
        29,
        0,
        0,
        365,
        193,
        "Page Setup",
        Some((8, "MS Shell Dlg")),
    );
    // Preview
    t.add(child(BS_GROUPBOX), 0, 240, 6, 120, 153, 0, "Button", "Preview");
    t.add(child(SS_WHITERECT), 0, 260, 42, 80, 80, DLG_RCT1, "Static", "");
    t.add(child(SS_GRAYRECT), 0, 340, 46, 4, 80, DLG_RCT2, "Static", "");
    t.add(child(SS_GRAYRECT), 0, 264, 122, 80, 4, DLG_RCT3, "Static", "");
    // Paper
    t.add(child(BS_GROUPBOX), 0, 8, 6, 224, 56, DLG_GRP2, "Button", "Paper");
    t.add(child(SS_LEFT | WS_GROUP), 0, 16, 22, 36, 8, DLG_STC2, "Static", "&Size:");
    t.add(
        child(CBS_SIMPLE | CBS_DROPDOWN | CBS_SORT | WS_GROUP | WS_TABSTOP | WS_VSCROLL),
        0, 64, 20, 160, 160, DLG_CMB2, "ComboBox", "",
    );
    t.add(child(SS_LEFT | WS_GROUP), 0, 16, 42, 36, 8, DLG_STC3, "Static", "&Tray:");
    t.add(
        child(CBS_SIMPLE | CBS_DROPDOWN | CBS_SORT | WS_GROUP | WS_TABSTOP | WS_VSCROLL),
        0, 64, 40, 160, 160, DLG_CMB3, "ComboBox", "",
    );
    // Orientation
    t.add(child(BS_GROUPBOX), 0, 8, 66, 64, 56, DLG_GRP1, "Button", "Orientation");
    t.add(child(BS_AUTORADIOBUTTON | WS_GROUP | WS_TABSTOP), 0, 16, 80, 52, 12, DLG_RAD1, "Button", "&Portrait");
    t.add(child(BS_AUTORADIOBUTTON), 0, 16, 100, 52, 12, DLG_RAD2, "Button", "&Landscape");
    // Borders
    t.add(child(BS_GROUPBOX), 0, 80, 66, 152, 56, DLG_GRP4, "Button", "Borders");
    t.add(child(SS_LEFT | WS_GROUP), 0, 88, 82, 30, 8, DLG_STC15, "Static", "L&eft:");
    t.add(child(ES_LEFT | WS_BORDER | WS_TABSTOP | WS_GROUP), 0, 119, 80, 36, 12, DLG_EDT4, "Edit", "");
    t.add(child(SS_LEFT | WS_GROUP), 0, 159, 82, 30, 8, DLG_STC16, "Static", "&Right:");
    t.add(child(ES_LEFT | WS_BORDER | WS_TABSTOP | WS_GROUP), 0, 190, 80, 36, 12, DLG_EDT6, "Edit", "");
    t.add(child(SS_LEFT | WS_GROUP), 0, 88, 102, 30, 8, DLG_STC17, "Static", "T&op:");
    t.add(child(ES_LEFT | WS_BORDER | WS_TABSTOP | WS_GROUP), 0, 119, 100, 36, 12, DLG_EDT5, "Edit", "");
    t.add(child(SS_LEFT | WS_GROUP), 0, 159, 102, 30, 8, DLG_STC18, "Static", "&Bottom:");
    t.add(child(ES_LEFT | WS_BORDER | WS_TABSTOP | WS_GROUP), 0, 190, 100, 36, 12, DLG_EDT7, "Edit", "");
    // Header / Footer
    t.add(child(SS_LEFT | WS_GROUP), 0, 8, 132, 40, 15, 0x140, "Static", "&Header:");
    t.add(child(ES_LEFT | WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL), 0, 58, 130, 173, 12, 0x141, "Edit", "");
    t.add(child(SS_LEFT | WS_GROUP), 0, 8, 149, 40, 15, 0x142, "Static", "&Footer:");
    t.add(child(ES_LEFT | WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL), 0, 58, 147, 173, 12, 0x143, "Edit", "");
    // Buttons
    t.add(child(BS_PUSHBUTTON | WS_TABSTOP), 0, 8, 170, 50, 14, IDHELP, "Button", "Help");
    t.add(child(BS_DEFPUSHBUTTON | WS_TABSTOP), 0, 198, 170, 50, 14, IDOK_ID, "Button", "OK");
    t.add(child(BS_PUSHBUTTON | WS_TABSTOP), 0, 254, 170, 50, 14, IDCANCEL_ID, "Button", "Cancel");
    t.add(child(BS_PUSHBUTTON | WS_TABSTOP), 0, 310, 170, 50, 14, DLG_PSH3, "Button", "P&rinter...");
    t.finish()
}




