//! main.rs — port of main.c (WinMain, NOTEPAD_WndProc, EDIT_WndProc,
//! menu dispatch, find/replace engine, command-line handling).

#![allow(non_snake_case)]

mod consts;
mod dialogs;
mod globals;
mod printing;
mod resources;
mod settings;
mod strings;
mod text;

use consts::*;
use globals::g;
use strings::{sw, Arg};
use windows::core::{PCWSTR, w};
use windows::Win32::Foundation::{
    GlobalFree, HGLOBAL, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM,
};
use windows::Win32::Globalization::GetUserDefaultUILanguage;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, HBRUSH, HGDIOBJ, InvalidateRect, UpdateWindow, COLOR_WINDOW,
};
use windows::Win32::Storage::FileSystem::GetFullPathNameW;
use windows::Win32::System::DataExchange::IsClipboardFormatAvailable;
use windows::Win32::System::Environment::GetCommandLineW;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{GetStartupInfoW, STARTUPINFOW};
use windows::Win32::UI::Controls::{EM_CANUNDO, EM_GETSEL, EM_REPLACESEL, EM_SCROLLCARET, EM_SETSEL};
use windows::Win32::UI::Controls::Dialogs::{
    FINDMSGSTRING, FINDREPLACEW, FR_DIALOGTERM, FR_DOWN, FR_FINDNEXT, FR_MATCHCASE, FR_REPLACE,
    FR_REPLACEALL, FR_WHOLEWORD, GetFileTitleW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_DOWN, VK_LEFT, VK_RIGHT, VK_UP};
use windows::Win32::UI::Shell::{
    DragAcceptFiles, DragFinish, DragQueryFileW, HDROP, SHAddToRecentDocs, SHARD_PATHW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, CheckMenuItem, CreateWindowExW, CW_USEDEFAULT, DefWindowProcW,
    DestroyAcceleratorTable, DestroyWindow, DispatchMessageW, EnableMenuItem, EN_CHANGE,
    EN_HSCROLL, EN_VSCROLL, GetClientRect, GetMenu, GetMessageW, GetWindowPlacement,
    GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GWLP_WNDPROC, HACCEL, HMENU, IDC_ARROW,
    IDYES, IsDialogMessageW, LoadCursorW, MB_ICONEXCLAMATION, MB_OK, MB_YESNO, MENU_ITEM_FLAGS,
    MESSAGEBOX_RESULT, MessageBoxW, MF_BYCOMMAND, MF_CHECKED, MF_ENABLED, MF_GRAYED,
    MF_UNCHECKED, MoveWindow, MSG, PostQuitMessage, RegisterClassExW, RegisterWindowMessageW,
    SendMessageW, SetMenu, SetProcessDefaultLayout, SetWindowLongPtrW, SetWindowPlacement,
    SW_SHOWDEFAULT, TranslateAcceleratorW, TranslateMessage, WINDOWPLACEMENT, WINDOW_EX_STYLE,
    WNDCLASS_STYLES, WNDCLASSEXW, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_DROPFILES,
    WM_ERASEBKGND, WM_INITMENUPOPUP, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONUP, WM_QUERYENDSESSION,
    WM_SETFOCUS, WM_SIZE, WS_OVERLAPPEDWINDOW,
};

const CLASS_NAME: PCWSTR = w!("Notepad");
const WINDOW_NAME: PCWSTR = w!("Notepad");


/* ------------------------------------------------------------------------ */
/* Small helpers                                                            */
/* ------------------------------------------------------------------------ */

/// Copy a NUL-terminated wide string into a fixed buffer (StringCchCopy).
fn copy_to(dst: &mut [u16], src: &[u16]) {
    let n = src.iter().position(|&c| c == 0).unwrap_or(src.len());
    let n = n.min(dst.len() - 1);
    dst[..n].copy_from_slice(&src[..n]);
    dst[n] = 0;
}

/// Read a NUL-terminated string from a wide pointer (lstrlenW + copy).
unsafe fn read_wide(p: windows::core::PWSTR) -> Vec<u16> {
    let mut v = Vec::new();
    if p.is_null() {
        return v;
    }
    let mut i = 0usize;
    loop {
        let c = *p.0.add(i);
        if c == 0 {
            break;
        }
        v.push(c);
        i += 1;
    }
    v
}

/// ASCII case folding used when FR_MATCHCASE is not set (the C _tcsnicmp uses
/// the C locale; non-ASCII characters compare case-sensitively here).
fn wide_lower(c: u16) -> u16 {
    if (0x41..=0x5A).contains(&c) {
        c + 0x20
    } else {
        c
    }
}

/// _istalnum(c) || c == '_' (ASCII, C locale).
fn is_word_char(c: u16) -> bool {
    (0x30..=0x39).contains(&c)
        || (0x41..=0x5A).contains(&c)
        || (0x61..=0x7A).contains(&c)
        || c == b'_' as u16
}

/* ------------------------------------------------------------------------ */
/* NOTEPAD_EnableSearchMenu                                                 */
/* ------------------------------------------------------------------------ */

pub unsafe fn enable_search_menu() {
    let b_empty = GetWindowTextLengthW(g().h_edit) == 0;
    let u_enable: MENU_ITEM_FLAGS = if b_empty {
        MF_BYCOMMAND | MF_GRAYED
    } else {
        MF_BYCOMMAND | MF_ENABLED
    };
    let _ = EnableMenuItem(g().h_menu, CMD_SEARCH, u_enable);
    let _ = EnableMenuItem(g().h_menu, CMD_SEARCH_NEXT, u_enable);
    let _ = EnableMenuItem(g().h_menu, CMD_SEARCH_PREV, u_enable);
}

/* ------------------------------------------------------------------------ */
/* SetFileName: sets the global file name, derives the file title and       */
/* registers the document in the recent files list.                         */
/* ------------------------------------------------------------------------ */

pub unsafe fn set_file_name(sz_file_name: &[u16]) {
    /* Detach from the caller's buffer first: dialogs.rs passes
     * &g().sz_file_name itself. */
    let mut name = sz_file_name.to_vec();
    if name.last().copied() != Some(0) {
        name.push(0);
    }

    let g = g();
    copy_to(&mut g.sz_file_name, &name);
    g.sz_file_title[0] = 0;
    if name[0] != 0 {
        GetFileTitleW(PCWSTR(name.as_ptr()), &mut g.sz_file_title);
        SHAddToRecentDocs(
            SHARD_PATHW.0 as u32,
            Some(name.as_ptr() as *const core::ffi::c_void),
        );
    }
}

/* ------------------------------------------------------------------------ */
/* NOTEPAD_MenuCommand: all handling of main menu events                    */
/* ------------------------------------------------------------------------ */

unsafe fn menu_command(w_param: WPARAM) {
    match (w_param.0 & 0xFFFF) as u32 {
        CMD_NEW => dialogs::dialog_file_new(),
        CMD_NEW_WINDOW => dialogs::dialog_file_new_window(),
        CMD_OPEN => dialogs::dialog_file_open(),
        CMD_SAVE => {
            dialogs::dialog_file_save();
        }
        CMD_SAVE_AS => {
            dialogs::dialog_file_save_as();
        }
        CMD_PRINT => printing::dialog_file_print(),
        CMD_PAGE_SETUP => printing::dialog_file_page_setup(),
        CMD_EXIT => dialogs::dialog_file_exit(),

        CMD_UNDO => dialogs::dialog_edit_undo(),
        CMD_CUT => dialogs::dialog_edit_cut(),
        CMD_COPY => dialogs::dialog_edit_copy(),
        CMD_PASTE => dialogs::dialog_edit_paste(),
        CMD_DELETE => dialogs::dialog_edit_delete(),
        CMD_SELECT_ALL => dialogs::dialog_edit_select_all(),
        CMD_TIME_DATE => dialogs::dialog_edit_time_date(),

        CMD_SEARCH => dialogs::dialog_search(),
        CMD_SEARCH_NEXT => dialogs::dialog_search_next(true),
        CMD_REPLACE => dialogs::dialog_replace(),
        CMD_GOTO => dialogs::dialog_goto(),
        CMD_SEARCH_PREV => dialogs::dialog_search_next(false),

        CMD_WRAP => dialogs::dialog_edit_wrap(),
        CMD_FONT => dialogs::dialog_select_font(),

        CMD_STATUSBAR => dialogs::dialog_view_status_bar(),

        CMD_HELP_CONTENTS => dialogs::dialog_help_contents(),
        CMD_HELP_ABOUT_NOTEPAD => dialogs::dialog_help_about_notepad(),

        _ => {}
    }
}

/* ------------------------------------------------------------------------ */
/* NOTEPAD_FindTextAt                                                       */
/* ------------------------------------------------------------------------ */

unsafe fn find_text_at(
    p_find_replace: &FINDREPLACEW,
    psz_text: &[u16],
    i_text_length: i32,
    dw_position: u32,
) -> bool {
    /* C: if (!pFindReplace || !pszText) return FALSE; */
    if p_find_replace.lpstrFindWhat.is_null() || i_text_length == 0 {
        return false;
    }

    let target = read_wide(p_find_replace.lpstrFindWhat);
    let i_target_length = target.len();
    let pos = dw_position as usize;
    if pos + i_target_length > psz_text.len() {
        return false;
    }

    /* Make proper comparison */
    let chunk = &psz_text[pos..pos + i_target_length];
    let mut b_matches = if (p_find_replace.Flags & FR_MATCHCASE).0 != 0 {
        chunk.iter().zip(target.iter()).all(|(a, b)| a == b)
    } else {
        chunk
            .iter()
            .zip(target.iter())
            .all(|(a, b)| wide_lower(*a) == wide_lower(*b))
    };

    if b_matches && (p_find_replace.Flags & FR_WHOLEWORD).0 != 0 {
        if dw_position > 0 && is_word_char(psz_text[pos - 1]) {
            b_matches = false;
        }
        if (dw_position as i32 + i_target_length as i32) < i_text_length
            && is_word_char(psz_text[pos + i_target_length])
        {
            b_matches = false;
        }
    }

    b_matches
}

/* ------------------------------------------------------------------------ */
/* NOTEPAD_FindNext                                                         */
/* ------------------------------------------------------------------------ */

pub unsafe fn find_next(
    p_find_replace: &mut FINDREPLACEW,
    b_replace: bool,
    b_show_alert: bool,
) -> bool {
    let i_target_length = read_wide(p_find_replace.lpstrFindWhat).len();

    /* Retrieve the window text */
    let i_text_length = GetWindowTextLengthW(g().h_edit);
    let mut psz_text: Vec<u16> = Vec::new();
    if i_text_length > 0 {
        psz_text = vec![0u16; i_text_length as usize + 1];
        GetWindowTextW(g().h_edit, &mut psz_text);
    }

    let mut dw_begin: u32 = 0;
    let mut dw_end: u32 = 0;
    SendMessageW(
        g().h_edit,
        EM_GETSEL,
        Some(WPARAM(&mut dw_begin as *mut u32 as usize)),
        Some(LPARAM(&mut dw_end as *mut u32 as isize)),
    );

    let mut i_adjustment: i64 = 0;
    if b_replace && dw_end.wrapping_sub(dw_begin) == i_target_length as u32 {
        if find_text_at(p_find_replace, &psz_text, i_text_length, dw_begin) {
            SendMessageW(
                g().h_edit,
                EM_REPLACESEL,
                Some(WPARAM(1)),
                Some(LPARAM(p_find_replace.lpstrReplaceWith.0 as isize)),
            );
            i_adjustment = read_wide(p_find_replace.lpstrReplaceWith).len() as i64
                - dw_end.wrapping_sub(dw_begin) as i64;
        }
    }

    let mut b_matches = false;
    let mut dw_position: u32;
    if (p_find_replace.Flags & FR_DOWN).0 != 0 {
        /* Find Down */
        dw_position = dw_end;
        while dw_position < i_text_length as u32 {
            b_matches = find_text_at(p_find_replace, &psz_text, i_text_length, dw_position);
            if b_matches {
                break;
            }
            dw_position += 1;
        }
    } else {
        /* Find Up */
        dw_position = dw_begin;
        while dw_position > 0 {
            dw_position -= 1;
            b_matches = find_text_at(p_find_replace, &psz_text, i_text_length, dw_position);
            if b_matches {
                break;
            }
        }
    }

    if b_matches {
        /* Found target */
        if dw_position > dw_begin {
            /* C: dwPosition += (DWORD)iAdjustment; (may wrap) */
            dw_position = (dw_position as i64 + i_adjustment) as u32;
        }
        SendMessageW(
            g().h_edit,
            EM_SETSEL,
            Some(WPARAM(dw_position as usize)),
            Some(LPARAM(
                dw_position.wrapping_add(i_target_length as u32) as usize as isize
            )),
        );
        SendMessageW(g().h_edit, EM_SCROLLCARET, Some(WPARAM(0)), Some(LPARAM(0)));
        true
    } else {
        /* Can't find target */
        if b_show_alert {
            let mut sz_find = read_wide(p_find_replace.lpstrFindWhat);
            sz_find.push(0);
            let sz_text = strings::format_sw(STRING_CANNOTFIND, &[Arg::S(&sz_find)]);
            let sz_caption = sw(STRING_NOTEPAD);
            MessageBoxW(
                Some(g().h_find_replace_dlg),
                PCWSTR(sz_text.as_ptr()),
                PCWSTR(sz_caption.as_ptr()),
                MB_OK,
            );
        }
        false
    }
}

/* ------------------------------------------------------------------------ */
/* NOTEPAD_ReplaceAll / NOTEPAD_FindTerm                                    */
/* ------------------------------------------------------------------------ */

unsafe fn replace_all(p_find_replace: &mut FINDREPLACEW) {
    let mut b_show_alert = true;

    SendMessageW(g().h_edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(0)));

    while find_next(p_find_replace, true, b_show_alert) {
        b_show_alert = false;
    }
}

unsafe fn find_term() {
    g().h_find_replace_dlg = HWND::default();
}

/* ------------------------------------------------------------------------ */
/* NOTEPAD_InitData: data initialization                                    */
/* ------------------------------------------------------------------------ */

unsafe fn init_data(h_instance: HINSTANCE) {
    /* Globals were zero-initialized by Globals::new() in win_main. */
    g().h_instance = h_instance;
    g().enc_file = ENCODING_DEFAULT;

    /* Build the open/save filter:
     *   <STRING_TEXT_FILES_TXT>\0*.txt\0<STRING_ALL_FILES>\0*.*\0\0 */
    let mut filter: Vec<u16> = Vec::new();
    filter.extend_from_slice(&sw(STRING_TEXT_FILES_TXT));
    filter.extend_from_slice(&strings::wide("*.txt"));
    filter.extend_from_slice(&sw(STRING_ALL_FILES));
    filter.extend_from_slice(&strings::wide("*.*"));
    filter.push(0);

    let g = g();
    let n = filter.len().min(g.sz_filter.len() - 1);
    g.sz_filter[..n].copy_from_slice(&filter[..n]);
    g.sz_filter[n] = 0;

    g.find.lpstrFindWhat = windows::core::PWSTR::null();
    g.h_dev_mode = HGLOBAL::default();
    g.h_dev_names = HGLOBAL::default();
}

/* ------------------------------------------------------------------------ */
/* NOTEPAD_InitMenuPopup: enable/disable items on the menu based on state   */
/* ------------------------------------------------------------------------ */

unsafe fn init_menu_popup(menu: HMENU, _index: LPARAM) {
    CheckMenuItem(
        menu,
        CMD_WRAP,
        if g().b_wrap_long_lines {
            MF_CHECKED.0
        } else {
            MF_UNCHECKED.0
        },
    );
    CheckMenuItem(
        menu,
        CMD_STATUSBAR,
        if g().b_show_status_bar {
            MF_CHECKED.0
        } else {
            MF_UNCHECKED.0
        },
    );
    let _ = EnableMenuItem(
        menu,
        CMD_UNDO,
        if SendMessageW(g().h_edit, EM_CANUNDO, Some(WPARAM(0)), Some(LPARAM(0))).0 != 0 {
            MF_ENABLED
        } else {
            MF_GRAYED
        },
    );
    let _ = EnableMenuItem(
        menu,
        CMD_PASTE,
        if IsClipboardFormatAvailable(1 /* CF_TEXT */).is_ok() {
            MF_ENABLED
        } else {
            MF_GRAYED
        },
    );

    let mut dw_start: u32 = 0;
    let mut dw_end: u32 = 0;
    SendMessageW(
        g().h_edit,
        EM_GETSEL,
        Some(WPARAM(&mut dw_start as *mut u32 as usize)),
        Some(LPARAM(&mut dw_end as *mut u32 as isize)),
    );
    let enable = if dw_start == dw_end {
        MF_GRAYED
    } else {
        MF_ENABLED
    };
    let _ = EnableMenuItem(menu, CMD_CUT, enable);
    let _ = EnableMenuItem(menu, CMD_COPY, enable);
    let _ = EnableMenuItem(menu, CMD_DELETE, enable);

    let _ = EnableMenuItem(
        menu,
        CMD_SELECT_ALL,
        if GetWindowTextLengthW(g().h_edit) != 0 {
            MF_ENABLED
        } else {
            MF_GRAYED
        },
    );
}

/* ------------------------------------------------------------------------ */
/* EDIT_WndProc: subclass procedure of the edit control                     */
/* ------------------------------------------------------------------------ */

pub unsafe extern "system" fn edit_wnd_proc(
    h_wnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_KEYDOWN | WM_KEYUP => {
            let vk = w_param.0 as u16;
            if vk == VK_UP.0 || vk == VK_DOWN.0 || vk == VK_LEFT.0 || vk == VK_RIGHT.0 {
                dialogs::status_bar_update_caret_pos();
            } else {
                dialogs::update_window_caption(false);
            }
            /* NOTE: the C switch falls through into the WM_LBUTTONUP case,
             * so the caret position is updated for every key event too. */
            dialogs::status_bar_update_caret_pos();
        }
        WM_LBUTTONUP => {
            dialogs::status_bar_update_caret_pos();
        }
        _ => {}
    }
    CallWindowProcW(g().edit_proc, h_wnd, msg, w_param, l_param)
}

/* ------------------------------------------------------------------------ */
/* NOTEPAD_WndProc: main window procedure                                   */
/* ------------------------------------------------------------------------ */

unsafe extern "system" fn main_wnd_proc(
    h_wnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            g().h_main_wnd = h_wnd;
            g().h_menu = GetMenu(h_wnd);

            /* The C original loads MAIN_MENU from the executable resources;
             * this port builds the menu in code (resources.rs). */
            if g().h_menu.is_invalid() {
                if let Ok(menu) = resources::load_main_menu() {
                    if SetMenu(h_wnd, Some(menu)).is_ok() {
                        g().h_menu = menu;
                    }
                }
            }

            DragAcceptFiles(h_wnd, true); /* Accept Drag & Drop */

            /* Create controls */
            dialogs::do_create_edit_window();
            dialogs::do_show_hide_status_bar();

            dialogs::dialog_file_new(); /* Initialize file info */

            /* For now, the "Help" dialog is disabled due to the lack of
             * HTML Help support */
            let _ = EnableMenuItem(g().h_menu, CMD_HELP_CONTENTS, MF_BYCOMMAND | MF_GRAYED);

            LRESULT(0)
        }

        WM_COMMAND => {
            let hiword = ((w_param.0 >> 16) & 0xFFFF) as u32;
            if hiword == EN_CHANGE || hiword == EN_HSCROLL || hiword == EN_VSCROLL {
                dialogs::status_bar_update_caret_pos();
            }
            if hiword == EN_CHANGE {
                enable_search_menu();
            }
            menu_command(WPARAM(w_param.0 & 0xFFFF));
            LRESULT(0)
        }

        WM_CLOSE => {
            if dialogs::do_close_file() {
                let _ = DestroyWindow(h_wnd);
            }
            LRESULT(0)
        }

        WM_QUERYENDSESSION => {
            if dialogs::do_close_file() {
                LRESULT(1)
            } else {
                LRESULT(0)
            }
        }

        WM_DESTROY => {
            let g = g();
            if !g.h_font.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(g.h_font.0));
            }
            if !g.h_dev_mode.is_invalid() {
                let _ = GlobalFree(Some(g.h_dev_mode));
            }
            if !g.h_dev_names.is_invalid() {
                let _ = GlobalFree(Some(g.h_dev_names));
            }
            let edit_proc = g.edit_proc;
            let h_edit = g.h_edit;
            if let Some(fp) = edit_proc {
                /* Restore the original window procedure */
                SetWindowLongPtrW(h_edit, GWLP_WNDPROC, fp as usize as isize);
            }
            settings::save_settings_to_registry();
            PostQuitMessage(0);
            LRESULT(0)
        }

        WM_SIZE => {
            let mut rc = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            let _ = GetClientRect(h_wnd, &mut rc);

            let b_status = g().b_show_status_bar;
            if b_status {
                let h_status = g().h_status_bar;
                SendMessageW(h_status, WM_SIZE, Some(WPARAM(0)), Some(LPARAM(0)));
                let mut rc_status = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                let _ = GetWindowRect(h_status, &mut rc_status);
                rc.bottom -= rc_status.bottom - rc_status.top;
            }

            let _ = MoveWindow(g().h_edit, 0, 0, rc.right, rc.bottom, true);

            if b_status {
                /* Align status bar parts, only if the status bar resize
                 * operation succeeds */
                dialogs::status_bar_align_parts();
            }
            LRESULT(0)
        }

        /* The entire client area is covered by edit control and by the
         * status bar. So there is no need to erase main background. This
         * resolves the horrible flicker effect during window resizes. */
        WM_ERASEBKGND => LRESULT(1),

        WM_SETFOCUS => {
            let _ = SetFocus(Some(g().h_edit));
            LRESULT(0)
        }

        WM_DROPFILES => {
            let h_drop = HDROP(w_param.0 as *mut core::ffi::c_void);
            let mut sz_file_name = [0u16; MAX_PATH_US];
            DragQueryFileW(h_drop, 0, Some(&mut sz_file_name));
            DragFinish(h_drop);
            dialogs::do_open_file(&sz_file_name);
            LRESULT(0)
        }

        WM_INITMENUPOPUP => {
            init_menu_popup(HMENU(w_param.0 as *mut core::ffi::c_void), l_param);
            LRESULT(0)
        }

        _ => {
            if msg == g().a_find_msg_string {
                /* Copy the FINDREPLACEW carried by the message
                 * (C: Globals.find = *(FINDREPLACE *)lParam) */
                g().find = std::ptr::read(l_param.0 as *const FINDREPLACEW);
                let mut fr: FINDREPLACEW = std::ptr::read(l_param.0 as *const FINDREPLACEW);

                dialogs::wait_cursor(true);

                if (fr.Flags & FR_FINDNEXT).0 != 0 {
                    find_next(&mut fr, false, true);
                } else if (fr.Flags & FR_REPLACE).0 != 0 {
                    find_next(&mut fr, true, true);
                } else if (fr.Flags & FR_REPLACEALL).0 != 0 {
                    replace_all(&mut fr);
                } else if (fr.Flags & FR_DIALOGTERM).0 != 0 {
                    find_term();
                }

                dialogs::wait_cursor(false);
                LRESULT(0)
            } else {
                DefWindowProcW(h_wnd, msg, w_param, l_param)
            }
        }
    }
}

/* ------------------------------------------------------------------------ */
/* AlertFileDoesNotExist                                                    */
/* ------------------------------------------------------------------------ */

unsafe fn alert_file_does_not_exist(sz_file_name: &[u16]) -> MESSAGEBOX_RESULT {
    dialogs::string_msg_box(
        Some(g().h_main_wnd),
        STRING_DOESNOTEXIST,
        sz_file_name,
        MB_ICONEXCLAMATION | MB_YESNO,
    )
}

/* ------------------------------------------------------------------------ */
/* HandleCommandLine                                                        */
/* ------------------------------------------------------------------------ */

unsafe fn handle_command_line(cmdline: &[u16]) -> bool {
    let mut opt_print = false;

    let len = cmdline.len();
    let mut i = 0usize;
    while i < len && cmdline[i] != 0 {
        let c = cmdline[i];
        if !(c == b' ' as u16 || c == b'-' as u16 || c == b'/' as u16) {
            break;
        }
        i += 1;
        if c == b' ' as u16 {
            continue;
        }
        let option = if i < len { cmdline[i] } else { 0 };
        if option != 0 {
            i += 1;
        }
        while i < len && cmdline[i] == b' ' as u16 {
            i += 1;
        }
        if option == b'p' as u16 || option == b'P' as u16 {
            opt_print = true;
        }
    }

    if i < len && cmdline[i] != 0 {
        /* file name is passed in the command line */
        let mut buf: Vec<u16> = Vec::new();

        /* Strip the quotes (C: cmdline++ and NUL out the last character) */
        let mut file_name: Vec<u16> = if cmdline[i] == b'"' as u16 {
            i += 1;
            let mut end = i;
            while end < len && cmdline[end] != 0 {
                end += 1;
            }
            let stop = if end > i { end - 1 } else { end };
            cmdline[i..stop].to_vec()
        } else {
            let mut end = i;
            while end < len && cmdline[end] != 0 {
                end += 1;
            }
            cmdline[i..end].to_vec()
        };

        let mut file_exists = dialogs::file_exists(&file_name);

        if !file_exists && !dialogs::has_file_extension(&file_name) {
            /* try to find file with ".txt" extension */
            let ends_with_txt = file_name.len() >= 4
                && file_name[file_name.len() - 4..]
                    .iter()
                    .copied()
                    .eq(".txt".encode_utf16());
            if !ends_with_txt {
                buf.extend_from_slice(&file_name);
                buf.truncate(MAX_PATH_US - 4 - 1);
                buf.extend(".txt".encode_utf16());
                buf.push(0);
                file_exists = dialogs::file_exists(&buf);
                file_name = buf.clone();
            }
        }

        /* GetFullPathName(file_name, _countof(szPath), szPath, NULL) */
        let mut file_name_z = file_name.to_vec();
        if file_name_z.last().copied() != Some(0) {
            file_name_z.push(0);
        }
        let mut sz_path = [0u16; MAX_PATH_US];
        GetFullPathNameW(PCWSTR(file_name_z.as_ptr()), Some(&mut sz_path), None);

        if file_exists {
            dialogs::do_open_file(&sz_path);
            let _ = InvalidateRect(Some(g().h_main_wnd), None, false);
            if opt_print {
                printing::dialog_file_print();
                return false;
            }
        } else {
            match alert_file_does_not_exist(&file_name_z) {
                IDYES => {
                    dialogs::do_open_file(&sz_path);
                }
                _ => {}
            }
        }
    }

    true
}

/* ------------------------------------------------------------------------ */
/* WinMain                                                                  */
/* ------------------------------------------------------------------------ */

/// Returns the part of the command line following the executable name
/// (the lpCmdLine parameter of WinMain).
unsafe fn get_command_line_tail() -> Vec<u16> {
    let mut v: Vec<u16> = Vec::new();
    let p = GetCommandLineW();
    if p.is_null() {
        v.push(0);
        return v;
    }
    let mut i = 0usize;
    /* skip the executable name (possibly quoted) */
    if *p.0 == b'"' as u16 {
        i = 1;
        while *p.0.add(i) != 0 && *p.0.add(i) != b'"' as u16 {
            i += 1;
        }
        if *p.0.add(i) == b'"' as u16 {
            i += 1;
        }
    } else {
        while *p.0.add(i) != 0 && *p.0.add(i) != b' ' as u16 && *p.0.add(i) != b'\t' as u16 {
            i += 1;
        }
    }
    /* copy the rest */
    while *p.0.add(i) != 0 {
        v.push(*p.0.add(i));
        i += 1;
    }
    v.push(0);
    v
}

unsafe fn win_main() -> i32 {
    /* C: switch (GetUserDefaultUILanguage()) — Hebrew UI gets RTL layout */
    if GetUserDefaultUILanguage() == 0x040D /* MAKELANGID(LANG_HEBREW, SUBLANG_DEFAULT) */ {
        let _ = SetProcessDefaultLayout(1 /* LAYOUT_RTL */);
    }

    /* NOTEPAD_InitData */
    globals::G = Some(globals::Globals::new());
    let h_instance = HINSTANCE(
        GetModuleHandleW(PCWSTR::null())
            .map(|h| h.0)
            .unwrap_or(std::ptr::null_mut()),
    );
    init_data(h_instance);

    /* aFINDMSGSTRING = RegisterWindowMessage(FINDMSGSTRING) */
    g().a_find_msg_string = RegisterWindowMessageW(FINDMSGSTRING);

    /* NOTEPAD_LoadSettingsFromRegistry(&wp) */
    let mut wp: WINDOWPLACEMENT = std::mem::zeroed();
    settings::load_settings_from_registry(&mut wp);

    /* Register the main window class */
    let wndclass = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: WNDCLASS_STYLES(0),
        lpfnWndProc: Some(main_wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: g().h_instance,
        hIcon: resources::load_icon().unwrap_or_default(),
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as *mut core::ffi::c_void),
        lpszMenuName: PCWSTR::null(), /* menu built in code (MAIN_MENU) */
        lpszClassName: CLASS_NAME,
        hIconSm: resources::load_small_icon().unwrap_or_default(),
    };
    g().h_icon = wndclass.hIcon;
    g().h_icon_sm = wndclass.hIconSm;
    if RegisterClassExW(&wndclass) == 0 {
        dialogs::show_last_error();
        return 1;
    }

    /* Globals.hMainWnd will be set in WM_CREATE handling */
    if CreateWindowExW(
        WINDOW_EX_STYLE(0),
        CLASS_NAME,
        WINDOW_NAME,
        WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        None,
        None,
        Some(g().h_instance),
        None,
    )
    .is_err()
    {
        dialogs::show_last_error();
        return 1;
    }
    if g().h_main_wnd.is_invalid() {
        dialogs::show_last_error();
        return 1;
    }

    /* Use the result of CW_USEDEFAULT if the data in the registry is not valid */
    if wp.rcNormalPosition.right == wp.rcNormalPosition.left {
        let _ = GetWindowPlacement(g().h_main_wnd, &mut wp);
    }
    /* Does the parent process want to force a show action? */
    let mut si: STARTUPINFOW = std::mem::zeroed();
    GetStartupInfoW(&mut si);
    let show = si.wShowWindow as i32;
    if show != SW_SHOWDEFAULT.0 {
        wp.showCmd = show as u32;
    }
    let _ = SetWindowPlacement(g().h_main_wnd, &wp);
    let _ = UpdateWindow(g().h_main_wnd);

    if !handle_command_line(&get_command_line_tail()) {
        return 0;
    }

    /* hAccel = LoadAccelerators(hInstance, MAKEINTRESOURCE(ID_ACCEL)) */
    let h_accel: HACCEL = resources::load_accelerators().unwrap_or_default();
    g().h_accel = h_accel;

    /* Message loop */
    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
        let h_find = g().h_find_replace_dlg;
        let dlg_handled = !h_find.is_invalid() && IsDialogMessageW(h_find, &msg).as_bool();
        if !dlg_handled && TranslateAcceleratorW(g().h_main_wnd, h_accel, &msg) == 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    let _ = DestroyAcceleratorTable(h_accel);

    msg.wParam.0 as i32
}

fn main() {
    let exit_code = unsafe { win_main() };
    std::process::exit(exit_code);
}
