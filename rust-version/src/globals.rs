//! NOTEPAD_GLOBALS — port of the Globals struct from notepad.h.
//! The whole application runs on a single UI thread (plus the printing
//! thread, which only reads), so a `static mut` faithfully mirrors the C.

#![allow(static_mut_refs)]

use crate::consts::{Encoding, Eoln, MAX_PATH_US};
use crate::strings::MAX_FILTER;
use windows::Win32::Foundation::{HGLOBAL, HINSTANCE, HWND, RECT};
use windows::Win32::Graphics::Gdi::{HFONT, LOGFONTW};
use windows::Win32::UI::Controls::Dialogs::FINDREPLACEW;
use windows::Win32::UI::WindowsAndMessaging::{HACCEL, HMENU, HICON, WNDPROC};

pub struct Globals {
    pub h_instance: HINSTANCE,
    pub h_main_wnd: HWND,
    pub h_find_replace_dlg: HWND,
    pub h_edit: HWND,
    pub h_status_bar: HWND,
    pub h_font: HFONT, // Font used by the edit control
    pub h_menu: HMENU,
    pub h_dev_mode: HGLOBAL,
    pub h_dev_names: HGLOBAL,
    pub h_icon: HICON,
    pub h_icon_sm: HICON,
    pub h_accel: HACCEL,
    pub lf_font: LOGFONTW,
    pub b_wrap_long_lines: bool,
    pub b_show_status_bar: bool,
    pub sz_find_text: [u16; MAX_PATH_US],
    pub sz_replace_text: [u16; MAX_PATH_US],
    pub sz_file_name: [u16; MAX_PATH_US],
    pub sz_file_title: [u16; MAX_PATH_US],
    pub sz_filter: [u16; MAX_FILTER],
    pub l_margins: RECT, // The margin values in 100th millimeters
    pub sz_header: [u16; MAX_PATH_US],
    pub sz_footer: [u16; MAX_PATH_US],
    pub sz_status_bar_line_col: [u16; MAX_PATH_US],

    pub enc_file: Encoding,
    pub i_eoln: Eoln,

    pub find: FINDREPLACEW,
    pub edit_proc: WNDPROC,
    pub b_was_modified: bool,
    pub a_find_msg_string: u32,
}

impl Globals {
    /// ZeroMemory(&Globals) + Globals.encFile = ENCODING_DEFAULT
    pub fn new() -> Box<Globals> {
        let mut g: Globals = unsafe { std::mem::zeroed() };
        g.enc_file = crate::consts::ENCODING_DEFAULT;
        g.i_eoln = crate::consts::Eoln::CrLf;
        Box::new(g)
    }
}

pub static mut G: Option<Box<Globals>> = None;

/// Access the globals (single-threaded UI, like the C original).
#[inline]
pub unsafe fn g() -> &'static mut Globals {
    match &mut *std::ptr::addr_of_mut!(G) {
        Some(b) => b,
        None => std::hint::unreachable_unchecked(),
    }
}
