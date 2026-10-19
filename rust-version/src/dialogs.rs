//! dialogs.rs — port of dialog.c (file open/save, edit commands, status bar,
//! Find/Replace, GoTo, font, help) plus the small helpers it owns.

#![allow(non_snake_case)]

use crate::consts::*;
use crate::globals::g;
use crate::strings::{self, Arg, format_str, sw};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    GetLastError, GENERIC_READ, GENERIC_WRITE, GlobalFree, HINSTANCE, HLOCAL, LPARAM, RECT, WPARAM,
};
use windows::Win32::Globalization::{
    GetDateFormatW, GetTimeFormatW, DATE_LONGDATE, LOCALE_USER_DEFAULT,
};
use windows::Win32::Graphics::Gdi::{CreateFontIndirectW, DeleteObject, HGDIOBJ, LOGFONTW};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, GetFileAttributesW, SetEndOfFile, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ,
    FILE_SHARE_WRITE, INVALID_FILE_ATTRIBUTES, OPEN_ALWAYS,
};
use windows::Win32::System::Diagnostics::Debug::{
    FormatMessageW, FORMAT_MESSAGE_ALLOCATE_BUFFER, FORMAT_MESSAGE_FROM_SYSTEM,
};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, LocalLock, LocalUnlock,
};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Controls::{
    CreateStatusWindowW, CCS_BOTTOM, SBARS_SIZEGRIP, SB_SETPARTS, SB_SETTEXTW, EM_EMPTYUNDOBUFFER,
    EM_GETHANDLE, EM_GETLINECOUNT, EM_GETMODIFY, EM_GETSEL, EM_LIMITTEXT, EM_LINEFROMCHAR,
    EM_LINEINDEX, EM_REPLACESEL, EM_SCROLLCARET, EM_SETHANDLE, EM_SETMODIFY, EM_SETSEL, EM_UNDO,
    NMHDR,
};
use windows::Win32::UI::Controls::Dialogs::{
    ChooseFontW, FindTextW, GetOpenFileNameW, GetSaveFileNameW, ReplaceTextW, CDN_FILEOK,
    CF_INITTOLOGFONTSTRUCT, CF_NOVERTFONTS, CF_SCREENFONTS, CHOOSEFONTW, FINDREPLACEW,
    OPENFILENAMEW, OFN_ENABLEHOOK, OFN_ENABLETEMPLATEHANDLE, OFN_EXPLORER, OFN_FILEMUSTEXIST,
    OFN_HIDEREADONLY, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, FR_DOWN,
};
use windows::Win32::UI::Shell::{ShellAboutW, ShellExecuteW, WinHelpW};
use windows::Win32::UI::WindowsAndMessaging::{
    CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CW_USEDEFAULT, DestroyWindow,
    DialogBoxIndirectParamW, DLGPROC, DLGTEMPLATE, EnableMenuItem, EndDialog, GetClientRect,
    GetDlgItem, GetDlgItemInt, GetWindowTextLengthW, GetWindowTextW, GWLP_WNDPROC, HCURSOR,
    HELP_INDEX, IDC_WAIT, IDNO, IDCANCEL, IDOK, IDYES, LoadCursorW, MB_ICONERROR,
    MB_ICONEXCLAMATION, MB_ICONMASK, MB_ICONQUESTION, MB_OK, MB_YESNOCANCEL, MESSAGEBOX_RESULT,
    MESSAGEBOX_STYLE, MessageBoxW, MF_ENABLED, MF_GRAYED, PostMessageW, SW_HIDE,
    SW_SHOWNOACTIVATE, SW_SHOWNORMAL, SendDlgItemMessageW, SendMessageW, SetCursor, SetDlgItemInt,
    SetWindowLongPtrW, SetWindowTextW, ShowWindow, WM_CLEAR, WM_CLOSE, WM_COMMAND,
    WM_COPY, WM_CUT, WM_INITDIALOG, WM_NOTIFY, WM_PASTE, WM_SETFONT, WM_SIZE, WNDPROC,
    WS_EX_CLIENTEDGE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;

// Status bar parts index
const SBPART_CURPOS: usize = 0;
const SBPART_EOLN: usize = 1;
const SBPART_ENCODING: usize = 2;

/* Line endings - string resource ID mapping table */
const EOLN_TO_STR_ID: [u32; 3] = [STRING_CRLF, STRING_LF, STRING_CR];
/* Encoding - string resource ID mapping table */
const ENC_TO_STR_ID: [u32; 5] = [
    STRING_ANSI,
    STRING_UNICODE,
    STRING_UNICODE_BE,
    STRING_UTF8,
    STRING_UTF8_BOM,
];

/// Copy a NUL-terminated wide string into a fixed buffer (StringCchCopy).
fn copy_to(dst: &mut [u16], src: &[u16]) {
    let n = src.iter().position(|&c| c == 0).unwrap_or(src.len());
    let n = n.min(dst.len() - 1);
    dst[..n].copy_from_slice(&src[..n]);
    dst[n] = 0;
}

/// Length of a NUL-terminated wide buffer.
fn wlen(buf: &[u16]) -> usize {
    buf.iter().position(|&c| c == 0).unwrap_or(buf.len())
}

pub unsafe fn show_last_error() {
    let error = GetLastError().0;
    if error != 0 {
        let sz_title = sw(STRING_ERROR);
        let mut lp_msg_buf: PWSTR = PWSTR::null();

        FormatMessageW(
            FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_SYSTEM,
            None,
            error,
            0,
            PWSTR(&mut lp_msg_buf as *mut PWSTR as *mut u16),
            0,
            None,
        );

        let mut psz_message: Vec<u16>;
        if !lp_msg_buf.is_null() {
            let cch = (0usize..).take_while(|&i| *lp_msg_buf.0.add(i) != 0).count();
            psz_message = std::slice::from_raw_parts(lp_msg_buf.0, cch).to_vec();
        } else {
            psz_message = format_str("%u", &[Arg::U(error)]);
        }
        psz_message.push(0);

        MessageBoxW(
            Some(g().h_main_wnd),
            PCWSTR(psz_message.as_ptr()),
            PCWSTR(sz_title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
        if !lp_msg_buf.is_null() {
            let _ = windows::Win32::Foundation::LocalFree(Some(HLOCAL(
                lp_msg_buf.0 as *mut core::ffi::c_void,
            )));
        }
    }
}

/// Sets the caption of the main window according to Globals.szFileTitle:
///    (untitled) - Notepad      if no file is open
///    [filename] - Notepad      if a file is given
pub unsafe fn update_window_caption(clear_modify_alert: bool) {
    let is_modified;
    if clear_modify_alert {
        /* When a file is being opened or created, there is no need to have
         * the edited flag shown when the file has not been edited yet. */
        is_modified = false;
    } else {
        /* Check whether the user has modified the file or not. If we are
         * in the same state as before, don't change the caption. */
        is_modified = SendMessageW(g().h_edit, EM_GETMODIFY, Some(WPARAM(0)), Some(LPARAM(0))).0 != 0;
        if is_modified == g().b_was_modified {
            return;
        }
    }

    /* Remember the state for later calls */
    g().b_was_modified = is_modified;

    /* Load the name of the application */
    let sz_notepad = sw(STRING_NOTEPAD);

    /* Determine if the file has been saved or if this is a new file */
    let sz_filename: Vec<u16> = if g().sz_file_title[0] != 0 {
        g().sz_file_title.to_vec()
    } else {
        sw(STRING_UNTITLED)
    };

    /* Update the window caption based upon whether the user has modified the file or not */
    let mut sz_caption: Vec<u16> = Vec::new();
    if is_modified {
        sz_caption.push('*' as u16);
    }
    sz_caption.extend_from_slice(&sz_filename[..wlen(&sz_filename)]);
    sz_caption.extend_from_slice(&" - ".encode_utf16().collect::<Vec<u16>>());
    sz_caption.extend_from_slice(&sz_notepad[..wlen(&sz_notepad)]);
    sz_caption.push(0);

    let _ = SetWindowTextW(g().h_main_wnd, PCWSTR(sz_caption.as_ptr()));
}

static mut S_H_WAIT_CURSOR: Option<HCURSOR> = None;
static mut S_H_OLD_CURSOR: Option<HCURSOR> = None;
static mut S_N_LOCK: i32 = 0;

pub unsafe fn wait_cursor(b_begin: bool) {
    if b_begin {
        S_N_LOCK += 1;
        if S_N_LOCK == 1 {
            let cur = S_H_WAIT_CURSOR;
            if cur.is_none() {
                S_H_WAIT_CURSOR = Some(LoadCursorW(None, IDC_WAIT).unwrap_or_default());
            }
            S_H_OLD_CURSOR = Some(SetCursor(S_H_WAIT_CURSOR));
        } else {
            SetCursor(S_H_WAIT_CURSOR);
        }
    } else {
        S_N_LOCK -= 1;
        if S_N_LOCK == 0 {
            SetCursor(S_H_OLD_CURSOR);
        }
    }
}

pub unsafe fn status_bar_align_parts() {
    let default_widths = [120i32, 120, 120];
    let mut rc_status_bar = RECT::default();
    let _ = GetClientRect(g().h_status_bar, &mut rc_status_bar);

    let mut parts = [0i32; 3];
    parts[0] = rc_status_bar.right - (default_widths[1] + default_widths[2]);
    parts[1] = rc_status_bar.right - default_widths[2];
    parts[2] = -1; // the right edge of the status bar

    parts[0] = parts[0].max(default_widths[0]);
    parts[1] = parts[1].max(default_widths[0] + default_widths[1]);

    SendMessageW(
        g().h_status_bar,
        SB_SETPARTS,
        Some(WPARAM(parts.len())),
        Some(LPARAM(parts.as_ptr() as isize)),
    );
}

unsafe fn status_bar_update_line_endings() {
    let sz_text = sw(EOLN_TO_STR_ID[g().i_eoln as usize]);
    SendMessageW(
        g().h_status_bar,
        SB_SETTEXTW,
        Some(WPARAM(SBPART_EOLN)),
        Some(LPARAM(sz_text.as_ptr() as isize)),
    );
}

unsafe fn status_bar_update_encoding() {
    let mut sz_text: Vec<u16> = Vec::new();
    if g().enc_file != Encoding::Auto {
        sz_text = sw(ENC_TO_STR_ID[g().enc_file as usize]);
    }
    SendMessageW(
        g().h_status_bar,
        SB_SETTEXTW,
        Some(WPARAM(SBPART_ENCODING)),
        Some(LPARAM(sz_text.as_ptr() as isize)),
    );
}

pub unsafe fn status_bar_update_all() {
    status_bar_update_caret_pos();
    status_bar_update_line_endings();
    status_bar_update_encoding();
}

pub unsafe fn string_msg_box(
    h_parent: Option<windows::Win32::Foundation::HWND>,
    format_id: u32,
    sz_string: &[u16],
    dw_flags: MESSAGEBOX_STYLE,
) -> MESSAGEBOX_RESULT {
    /* Load and format szMessage */
    let sz_message = crate::strings::format_sw(format_id, &[Arg::S(sz_string)]);

    /* Load szCaption */
    let sz_resource = if (dw_flags & MB_ICONMASK) == MB_ICONEXCLAMATION {
        sw(STRING_ERROR)
    } else {
        sw(STRING_NOTEPAD)
    };

    /* Display Modal Dialog */
    MessageBoxW(
        h_parent,
        PCWSTR(sz_message.as_ptr()),
        PCWSTR(sz_resource.as_ptr()),
        dw_flags,
    )
}

unsafe fn alert_file_not_found(sz_file_name: &[u16]) {
    string_msg_box(
        Some(g().h_main_wnd),
        STRING_NOTFOUND,
        sz_file_name,
        MB_ICONEXCLAMATION | MB_OK,
    );
}

unsafe fn alert_file_not_saved(sz_file_name: &[u16]) -> MESSAGEBOX_RESULT {
    let sz_untitled = sw(STRING_UNTITLED);
    let name: &[u16] = if sz_file_name.first().copied().unwrap_or(0) != 0 {
        sz_file_name
    } else {
        &sz_untitled
    };
    string_msg_box(
        Some(g().h_main_wnd),
        STRING_NOTSAVED,
        name,
        MB_ICONQUESTION | MB_YESNOCANCEL,
    )
}

/// Returns: TRUE if file exists, FALSE if file does not exist
pub unsafe fn file_exists(sz_filename: &[u16]) -> bool {
    let mut name = sz_filename.to_vec();
    name.push(0);
    GetFileAttributesW(PCWSTR(name.as_ptr())) != INVALID_FILE_ATTRIBUTES
}

pub unsafe fn has_file_extension(sz_filename: &[u16]) -> bool {
    let mut start = 0usize;
    for (i, &c) in sz_filename.iter().take_while(|&&c| c != 0).enumerate() {
        if c == '\\' as u16 {
            start = i + 1;
        }
    }
    sz_filename[start..]
        .iter()
        .take_while(|&&c| c != 0)
        .any(|&c| c == '.' as u16)
}

unsafe fn do_save_file() -> bool {
    let mut b_ret = false;

    wait_cursor(true);

    /* Use OPEN_ALWAYS instead of CREATE_ALWAYS in order to succeed
     * even if the file has HIDDEN or SYSTEM attributes */
    let mut name = g().sz_file_name.to_vec();
    name.push(0);
    let h_file = CreateFileW(
        PCWSTR(name.as_ptr()),
        GENERIC_WRITE.0,
        FILE_SHARE_READ,
        None,
        OPEN_ALWAYS,
        FILE_ATTRIBUTE_NORMAL,
        None,
    );
    let h_file = match h_file {
        Ok(h) => h,
        Err(_) => {
            show_last_error();
            wait_cursor(false);
            return false;
        }
    };

    let cch_text = GetWindowTextLengthW(g().h_edit);
    if cch_text <= 0 {
        b_ret = true;
    } else {
        let h_local =
            HLOCAL(SendMessageW(g().h_edit, EM_GETHANDLE, Some(WPARAM(0)), Some(LPARAM(0))).0 as *mut core::ffi::c_void);
        let psz_text = LocalLock(h_local) as *const u16;
        if !psz_text.is_null() {
            let slice = std::slice::from_raw_parts(psz_text, cch_text as usize);
            match crate::text::write_text(h_file, slice, g().enc_file, g().i_eoln) {
                Ok(()) => b_ret = true,
                Err(_) => show_last_error(),
            }
            let _ = LocalUnlock(h_local);
        } else {
            show_last_error();
        }
    }

    /* Truncate the file and close it */
    let _ = SetEndOfFile(h_file);
    let _ = windows::Win32::Foundation::CloseHandle(h_file);

    if b_ret {
        SendMessageW(g().h_edit, EM_SETMODIFY, Some(WPARAM(0)), Some(LPARAM(0)));
        crate::set_file_name(&g().sz_file_name);
    }

    wait_cursor(false);
    b_ret
}

/// Returns:
///   TRUE  - User agreed to close (both save/don't save)
///   FALSE - User cancelled close by selecting "Cancel"
pub unsafe fn do_close_file() -> bool {
    if SendMessageW(g().h_edit, EM_GETMODIFY, Some(WPARAM(0)), Some(LPARAM(0))).0 != 0 {
        /* prompt user to save changes */
        let n_result = alert_file_not_saved(&g().sz_file_name);
        match n_result {
            IDYES => {
                if !dialog_file_save() {
                    return false;
                }
            }
            IDNO => {}
            _ => return false, // IDCANCEL and default
        }
    }

    crate::set_file_name(&[]);
    update_window_caption(true);

    true
}

pub unsafe fn do_open_file(sz_file_name: &[u16]) {
    let mut h_file: Option<windows::Win32::Foundation::HANDLE> = None;

    /* Close any files and prompt to save changes */
    if !do_close_file() {
        return;
    }

    wait_cursor(true);
    let _ = SetWindowTextW(g().h_edit, PCWSTR::null());

    'done: {
        let mut name = sz_file_name.to_vec();
        name.push(0);
        h_file = match CreateFileW(
            PCWSTR(name.as_ptr()),
            GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_ALWAYS,
            FILE_ATTRIBUTE_NORMAL,
            None,
        ) {
            Ok(h) => Some(h),
            Err(_) => {
                show_last_error();
                break 'done;
            }
        };
        let h_file = h_file.unwrap();

        /* To make loading file quicker, we use the internal handle of EDIT control */
        let h_old_local = HLOCAL(
            SendMessageW(g().h_edit, EM_GETHANDLE, Some(WPARAM(0)), Some(LPARAM(0))).0 as *mut core::ffi::c_void,
        );
        let mut enc_file = g().enc_file;
        let mut i_eoln = g().i_eoln;
        let h_new_local = match crate::text::read_text(h_file, &mut enc_file, &mut i_eoln) {
            Some(h) => h,
            None => {
                show_last_error();
                break 'done;
            }
        };
        g().enc_file = enc_file;
        g().i_eoln = i_eoln;
        SendMessageW(g().h_edit, EM_SETHANDLE, Some(WPARAM(h_new_local.0 as usize)), Some(LPARAM(0)));
        let _ = windows::Win32::Foundation::LocalFree(Some(h_old_local));
        /* No need of EM_SETMODIFY and EM_EMPTYUNDOBUFFER here. EM_SETHANDLE does instead. */

        let _ = SetFocus(Some(g().h_edit));

        /*  If the file starts with .LOG, add a time/date at the end and set cursor after
         *  See http://web.archive.org/web/20090627165105/http://support.microsoft.com/kb/260563
         */
        let mut log = [0u16; 5];
        let n = GetWindowTextW(g().h_edit, &mut log);
        let dot_log: Vec<u16> = ".LOG".encode_utf16().collect();
        if n >= 4 && log[..4] == dot_log[..] {
            let lf: Vec<u16> = "\r\n".encode_utf16().chain(std::iter::once(0)).collect();
            let len = GetWindowTextLengthW(g().h_edit);
            SendMessageW(g().h_edit, EM_SETSEL, Some(WPARAM(len as usize)), Some(LPARAM(-1)));
            SendMessageW(g().h_edit, EM_REPLACESEL, Some(WPARAM(1)), Some(LPARAM(lf.as_ptr() as isize)));
            dialog_edit_time_date();
            SendMessageW(g().h_edit, EM_REPLACESEL, Some(WPARAM(1)), Some(LPARAM(lf.as_ptr() as isize)));
        }

        crate::set_file_name(sz_file_name);
        update_window_caption(true);
        crate::enable_search_menu();
        status_bar_update_all();
    }

    /* done: */
    if let Some(h) = h_file {
        let _ = windows::Win32::Foundation::CloseHandle(h);
    }
    wait_cursor(false);
}

pub unsafe fn dialog_file_new() {
    /* Close any files and prompt to save changes */
    if !do_close_file() {
        return;
    }

    wait_cursor(true);

    let _ = SetWindowTextW(g().h_edit, PCWSTR::null());
    SendMessageW(g().h_edit, EM_EMPTYUNDOBUFFER, Some(WPARAM(0)), Some(LPARAM(0)));
    g().i_eoln = Eoln::CrLf;
    g().enc_file = ENCODING_DEFAULT;

    crate::enable_search_menu();
    status_bar_update_all();

    wait_cursor(false);
}

pub unsafe fn dialog_file_new_window() {
    wait_cursor(true);

    let mut psz_notepad_exe = [0u16; MAX_PATH_US];
    let _ = GetModuleFileNameW(None, &mut psz_notepad_exe);
    let _ = ShellExecuteW(
        None,
        PCWSTR::null(),
        PCWSTR(psz_notepad_exe.as_ptr()),
        PCWSTR::null(),
        PCWSTR::null(),
        SW_SHOWNORMAL,
    );

    wait_cursor(false);
}

pub unsafe fn dialog_file_open() {
    let mut sz_path = [0u16; MAX_PATH_US];

    if g().sz_file_name[0] == 0 {
        copy_to(&mut sz_path, &"*.txt".encode_utf16().collect::<Vec<u16>>());
    } else {
        copy_to(&mut sz_path, &g().sz_file_name);
    }

    let mut openfilename: OPENFILENAMEW = std::mem::zeroed();
    openfilename.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    openfilename.hwndOwner = g().h_main_wnd;
    openfilename.lpstrFilter = PCWSTR(g().sz_filter.as_ptr());
    openfilename.lpstrFile = PWSTR(sz_path.as_mut_ptr());
    openfilename.nMaxFile = sz_path.len() as u32;
    openfilename.Flags =
        OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY;
    let sz_default_ext = strings::wide("txt");
    openfilename.lpstrDefExt = PCWSTR(sz_default_ext.as_ptr());

    if GetOpenFileNameW(&mut openfilename).as_bool() {
        if file_exists(&sz_path) {
            do_open_file(&sz_path);
        } else {
            alert_file_not_found(&sz_path);
        }
    }
}

pub unsafe fn dialog_file_save() -> bool {
    if g().sz_file_name[0] == 0 {
        dialog_file_save_as()
    } else if do_save_file() {
        update_window_caption(true);
        true
    } else {
        false
    }
}

unsafe fn fill_combo(h_dlg: windows::Win32::Foundation::HWND, id: i32, ids: &[u32], sel: usize) {
    if let Ok(h_combo) = GetDlgItem(Some(h_dlg), id) {
        for &string_id in ids {
            let sz_text = sw(string_id);
            SendMessageW(h_combo, CB_ADDSTRING, Some(WPARAM(0)), Some(LPARAM(sz_text.as_ptr() as isize)));
        }
        SendMessageW(h_combo, CB_SETCURSEL, Some(WPARAM(sel)), Some(LPARAM(0)));
    }
}

unsafe extern "system" fn save_as_hook(
    h_dlg: windows::Win32::Foundation::HWND,
    msg: u32,
    _wparam: WPARAM,
    lparam: LPARAM,
) -> usize {
    match msg {
        WM_INITDIALOG => {
            fill_combo(
                h_dlg,
                ID_ENCODING as i32,
                &ENC_TO_STR_ID,
                g().enc_file as usize,
            );
            fill_combo(
                h_dlg,
                ID_EOLN as i32,
                &EOLN_TO_STR_ID,
                g().i_eoln as usize,
            );
        }
        WM_NOTIFY => {
            let nmhdr = &*(lparam.0 as *const NMHDR);
            if nmhdr.code == CDN_FILEOK {
                if let Ok(h_combo) = GetDlgItem(Some(h_dlg), ID_ENCODING as i32) {
                    let sel = SendMessageW(h_combo, CB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0)));
                    g().enc_file = Encoding::from_i32(sel.0 as i32);
                }
                if let Ok(h_combo) = GetDlgItem(Some(h_dlg), ID_EOLN as i32) {
                    let sel = SendMessageW(h_combo, CB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0)));
                    g().i_eoln = Eoln::from_u32(sel.0 as u32);
                }
            }
        }
        _ => {}
    }
    0
}

pub unsafe fn dialog_file_save_as() -> bool {
    let mut sz_path = [0u16; MAX_PATH_US];

    if g().sz_file_name[0] == 0 {
        copy_to(&mut sz_path, &"*.txt".encode_utf16().collect::<Vec<u16>>());
    } else {
        copy_to(&mut sz_path, &g().sz_file_name);
    }

    /* Build the Encoding child dialog template at runtime and hand it to
     * comdlg32 through OFN_ENABLETEMPLATEHANDLE (the original binary used a
     * resource; we only have runtime-built templates). */
    let template = crate::resources::encoding_dialog_template();
    let h_template = GlobalAlloc(GMEM_MOVEABLE, template.len()).unwrap_or_default();
    let p_lock = GlobalLock(h_template);
    if !p_lock.is_null() {
        std::ptr::copy_nonoverlapping(template.as_ptr(), p_lock as *mut u8, template.len());
        let _ = GlobalUnlock(h_template);
    }

    let mut saveas: OPENFILENAMEW = std::mem::zeroed();
    saveas.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    saveas.hwndOwner = g().h_main_wnd;
    saveas.hInstance = HINSTANCE(h_template.0 as *mut core::ffi::c_void);
    saveas.lpstrFilter = PCWSTR(g().sz_filter.as_ptr());
    saveas.lpstrFile = PWSTR(sz_path.as_mut_ptr());
    saveas.nMaxFile = sz_path.len() as u32;
    saveas.Flags = OFN_PATHMUSTEXIST
        | OFN_OVERWRITEPROMPT
        | OFN_HIDEREADONLY
        | OFN_EXPLORER
        | OFN_ENABLETEMPLATEHANDLE
        | OFN_ENABLEHOOK;
    let sz_default_ext = strings::wide("txt");
    saveas.lpstrDefExt = PCWSTR(sz_default_ext.as_ptr());
    saveas.lpfnHook = Some(save_as_hook);

    let ok = GetSaveFileNameW(&mut saveas).as_bool();
    let _ = GlobalFree(Some(h_template));

    if ok {
        /* HACK: Because in ROS, Save-As boxes don't check the validity
         * of file names and thus, here, szPath can be invalid !! We only
         * see its validity when we call DoSaveFile()... */
        crate::set_file_name(&sz_path);
        if do_save_file() {
            update_window_caption(true);
            status_bar_update_all();
            true
        } else {
            crate::set_file_name(&[]);
            false
        }
    } else {
        false
    }
}

pub unsafe fn dialog_file_exit() {
    let _ = PostMessageW(Some(g().h_main_wnd), WM_CLOSE, WPARAM(0), LPARAM(0));
}

pub unsafe fn dialog_edit_undo() {
    SendMessageW(g().h_edit, EM_UNDO, Some(WPARAM(0)), Some(LPARAM(0)));
}

pub unsafe fn dialog_edit_cut() {
    SendMessageW(g().h_edit, WM_CUT, Some(WPARAM(0)), Some(LPARAM(0)));
}

pub unsafe fn dialog_edit_copy() {
    SendMessageW(g().h_edit, WM_COPY, Some(WPARAM(0)), Some(LPARAM(0)));
}

pub unsafe fn dialog_edit_paste() {
    SendMessageW(g().h_edit, WM_PASTE, Some(WPARAM(0)), Some(LPARAM(0)));
}

pub unsafe fn dialog_edit_delete() {
    SendMessageW(g().h_edit, WM_CLEAR, Some(WPARAM(0)), Some(LPARAM(0)));
}

pub unsafe fn dialog_edit_select_all() {
    SendMessageW(g().h_edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
}

pub unsafe fn dialog_edit_time_date() {
    let st = GetLocalTime();
    let mut sz_date = [0u16; MAX_STRING_LEN];
    let mut sz_text: Vec<u16> = Vec::new();

    GetTimeFormatW(LOCALE_USER_DEFAULT, 0, Some(&st), PCWSTR::null(), Some(&mut sz_date));
    sz_text.extend_from_slice(&sz_date[..wlen(&sz_date)]);
    sz_text.push(' ' as u16);
    GetDateFormatW(
        LOCALE_USER_DEFAULT,
        DATE_LONGDATE.0,
        Some(&st),
        PCWSTR::null(),
        Some(&mut sz_date),
    );
    sz_text.extend_from_slice(&sz_date[..wlen(&sz_date)]);
    sz_text.push(0);

    SendMessageW(g().h_edit, EM_REPLACESEL, Some(WPARAM(1)), Some(LPARAM(sz_text.as_ptr() as isize)));
}

pub unsafe fn do_show_hide_status_bar() {
    /* Check if status bar object already exists. */
    if g().b_show_status_bar && g().h_status_bar.is_invalid() {
        /* Try to create the status bar */
        let style = (WS_CHILD | SBARS_SIZEGRIP | CCS_BOTTOM as u32) as i32;
        match CreateStatusWindowW(style, PCWSTR::null(), g().h_main_wnd, CMD_STATUSBAR_WND_ID) {
            Ok(h) => g().h_status_bar = h,
            Err(_) => {
                show_last_error();
                return;
            }
        }

        /* Load the string for formatting column/row text output */
        let s = sw(STRING_LINE_COLUMN);
        copy_to(&mut g().sz_status_bar_line_col, &s);
    }

    /* Update layout of controls */
    SendMessageW(g().h_main_wnd, WM_SIZE, Some(WPARAM(0)), Some(LPARAM(0)));

    if g().h_status_bar.is_invalid() {
        return;
    }

    /* Update visibility of status bar */
    let _ = ShowWindow(
        g().h_status_bar,
        if g().b_show_status_bar {
            SW_SHOWNOACTIVATE
        } else {
            SW_HIDE
        },
    );

    /* Update status bar contents */
    status_bar_update_all();
}

pub unsafe fn do_create_edit_window() {
    let mut sz_temp: Option<Vec<u16>> = None;
    let mut b_modified = false;

    /* If the edit control already exists, try to save its content */
    if !g().h_edit.is_invalid() {
        /* number of chars currently written into the editor. */
        let i_size = GetWindowTextLengthW(g().h_edit);
        if i_size != 0 {
            /* Allocates temporary buffer. Recover the text into the control. */
            let mut buf = vec![0u16; i_size as usize + 1];
            GetWindowTextW(g().h_edit, &mut buf);
            sz_temp = Some(buf);

            if SendMessageW(g().h_edit, EM_GETMODIFY, Some(WPARAM(0)), Some(LPARAM(0))).0 != 0 {
                b_modified = true;
            }
        }

        /* Restore original window procedure */
        let old_proc: WNDPROC = g().edit_proc;
        if let Some(fp) = old_proc {
            SetWindowLongPtrW(g().h_edit, GWLP_WNDPROC, fp as usize as isize);
        }

        /* Destroy the edit control */
        let _ = DestroyWindow(g().h_edit);
    }

    /* Update wrap status into the main menu and recover style flags */
    let dw_style = if g().b_wrap_long_lines {
        EDIT_STYLE_WRAP
    } else {
        EDIT_STYLE
    };

    /* Create the new edit control */
    let edit_class = strings::wide(EDIT_CLASS);
    match windows::Win32::UI::WindowsAndMessaging::CreateWindowExW(
        WS_EX_CLIENTEDGE,
        PCWSTR(edit_class.as_ptr()),
        PCWSTR::null(),
        windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(dw_style),
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        Some(g().h_main_wnd),
        None,
        Some(g().h_instance),
        None,
    ) {
        Ok(h) => g().h_edit = h,
        Err(_) => {
            show_last_error();
            return;
        }
    }

    SendMessageW(g().h_edit, WM_SETFONT, Some(WPARAM(g().h_font.0 as usize)), Some(LPARAM(0)));
    SendMessageW(g().h_edit, EM_LIMITTEXT, Some(WPARAM(0)), Some(LPARAM(0)));

    /* If some text was previously saved, restore it. */
    if let Some(buf) = sz_temp {
        let _ = SetWindowTextW(g().h_edit, PCWSTR(buf.as_ptr()));
        if b_modified {
            SendMessageW(g().h_edit, EM_SETMODIFY, Some(WPARAM(1)), Some(LPARAM(0)));
        }
    }

    /* Sub-class a new window callback for row/column detection. */
    let new_proc: WNDPROC = Some(crate::edit_wnd_proc);
    let prev = SetWindowLongPtrW(g().h_edit, GWLP_WNDPROC, new_proc.unwrap() as usize as isize);
    g().edit_proc = std::mem::transmute::<isize, WNDPROC>(prev);

    /* Finally shows new edit control and set focus into it. */
    let _ = ShowWindow(g().h_edit, windows::Win32::UI::WindowsAndMessaging::SW_SHOW);
    let _ = SetFocus(Some(g().h_edit));

    /* Re-arrange controls */
    let _ = PostMessageW(Some(g().h_main_wnd), WM_SIZE, WPARAM(0), LPARAM(0));
}

pub unsafe fn dialog_edit_wrap() {
    g().b_wrap_long_lines = !g().b_wrap_long_lines;

    let _ = EnableMenuItem(
        g().h_menu,
        CMD_GOTO,
        if g().b_wrap_long_lines { MF_GRAYED } else { MF_ENABLED },
    );

    do_create_edit_window();
    do_show_hide_status_bar();
}

pub unsafe fn dialog_select_font() {
    let mut lf: LOGFONTW = g().lf_font;

    let mut cf: CHOOSEFONTW = std::mem::zeroed();
    cf.lStructSize = std::mem::size_of::<CHOOSEFONTW>() as u32;
    cf.hwndOwner = g().h_main_wnd;
    cf.lpLogFont = &mut lf;
    cf.Flags = CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_NOVERTFONTS;

    if ChooseFontW(&mut cf).as_bool() {
        let currfont = g().h_font;

        g().h_font = CreateFontIndirectW(&lf);
        g().lf_font = lf;
        SendMessageW(g().h_edit, WM_SETFONT, Some(WPARAM(g().h_font.0 as usize)), Some(LPARAM(1)));
        if !currfont.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(currfont.0));
        }
    }
}

unsafe fn search_dialog(b_find: bool) {
    if !g().h_find_replace_dlg.is_invalid() {
        let _ = SetFocus(Some(g().h_find_replace_dlg));
        return;
    }

    if g().find.lpstrFindWhat.is_null() {
        let g = g();
        g.find = std::mem::zeroed();
        g.find.lStructSize = std::mem::size_of::<FINDREPLACEW>() as u32;
        g.find.hwndOwner = g.h_main_wnd;
        g.find.lpstrFindWhat = PWSTR(g.sz_find_text.as_mut_ptr());
        g.find.wFindWhatLen = g.sz_find_text.len() as u16;
        g.find.lpstrReplaceWith = PWSTR(g.sz_replace_text.as_mut_ptr());
        g.find.wReplaceWithLen = g.sz_replace_text.len() as u16;
        g.find.Flags = FR_DOWN;
    }

    /* We only need to create the modeless FindReplace dialog which will */
    /* notify us of incoming events using hMainWnd Window Messages       */
    let g = g();
    g.h_find_replace_dlg = if b_find {
        FindTextW(&mut g.find)
    } else {
        ReplaceTextW(&mut g.find)
    };
    debug_assert!(!g.h_find_replace_dlg.is_invalid());
}

pub unsafe fn dialog_search() {
    search_dialog(true);
}

pub unsafe fn dialog_search_next(b_down: bool) {
    let g = g();
    if b_down {
        g.find.Flags |= FR_DOWN;
    } else {
        g.find.Flags &= !FR_DOWN;
    }

    if !g.find.lpstrFindWhat.is_null() && *g.find.lpstrFindWhat.0 != 0 {
        crate::find_next(&mut g.find, false, true);
    } else {
        dialog_search();
    }
}

pub unsafe fn dialog_replace() {
    search_dialog(false);
}

#[repr(C)]
struct GotoData {
    i_line: u32,
    c_lines: u32,
}

static mut S_P_GOTO_DATA: Option<*mut GotoData> = None;

unsafe extern "system" fn goto_dialog_proc(
    hwnd_dialog: windows::Win32::Foundation::HWND,
    u_msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> isize {
    match u_msg {
        WM_INITDIALOG => {
            S_P_GOTO_DATA = Some(l_param.0 as *mut GotoData);
            let data = &*S_P_GOTO_DATA.unwrap();
            let _ = SetDlgItemInt(hwnd_dialog, ID_LINENUMBER as i32, data.i_line, false);
            1 /* Set focus */
        }
        WM_COMMAND => {
            let cmd = (w_param.0 & 0xFFFF) as u32;
            if cmd == IDOK.0 as u32 {
                let data = &mut *S_P_GOTO_DATA.unwrap();
                let i_line = GetDlgItemInt(hwnd_dialog, ID_LINENUMBER as i32, None, false);
                if i_line == 0 || data.c_lines < i_line {
                    /* Out of range - Show error message */
                    let title = sw(STRING_NOTEPAD);
                    let text = sw(STRING_LINE_NUMBER_OUT_OF_RANGE);
                    let _ = MessageBoxW(
                        Some(hwnd_dialog),
                        PCWSTR(text.as_ptr()),
                        PCWSTR(title.as_ptr()),
                        MB_OK,
                    );

                    SendDlgItemMessageW(
                        hwnd_dialog,
                        ID_LINENUMBER as i32,
                        EM_SETSEL,
                        WPARAM(0),
                        LPARAM(-1),
                    );
                    if let Ok(h_edit) = GetDlgItem(Some(hwnd_dialog), ID_LINENUMBER as i32) {
                        let _ = SetFocus(Some(h_edit));
                    }
                } else {
                    data.i_line = i_line;
                    let _ = EndDialog(hwnd_dialog, IDOK.0 as isize);
                }
            } else if cmd == IDCANCEL.0 as u32 {
                let _ = EndDialog(hwnd_dialog, IDCANCEL.0 as isize);
            }
            0
        }
        _ => 0,
    }
}

pub unsafe fn dialog_goto() {
    let mut goto_data = GotoData {
        i_line: 0,
        c_lines: 0,
    };
    let mut dw_start: u32 = 0;
    let mut dw_end: u32 = 0;
    let cch = GetWindowTextLengthW(g().h_edit);

    /* Get the current line number and the total line number */
    SendMessageW(
        g().h_edit,
        EM_GETSEL,
        Some(WPARAM(&mut dw_start as *mut u32 as usize)),
        Some(LPARAM(&mut dw_end as *mut u32 as isize)),
    );
    goto_data.i_line =
        SendMessageW(g().h_edit, EM_LINEFROMCHAR, Some(WPARAM(dw_start as usize)), Some(LPARAM(0))).0 as u32 + 1;
    goto_data.c_lines = SendMessageW(g().h_edit, EM_GETLINECOUNT, Some(WPARAM(0)), Some(LPARAM(0))).0 as u32;

    /* Ask the user for line number */
    let template = crate::resources::goto_dialog_template();
    /* Copy into a u32-aligned buffer: DLGTEMPLATE requires DWORD alignment. */
    let mut aligned = vec![0u32; template.len().div_ceil(4)];
    let bytes = std::slice::from_raw_parts_mut(
        aligned.as_mut_ptr() as *mut u8,
        aligned.len() * 4,
    );
    bytes[..template.len()].copy_from_slice(&template);

    let ret = DialogBoxIndirectParamW(
        Some(g().h_instance),
        aligned.as_ptr() as *const DLGTEMPLATE,
        Some(g().h_main_wnd),
        {
            let proc: DLGPROC = Some(goto_dialog_proc);
            proc
        },
        LPARAM(&mut goto_data as *mut GotoData as isize),
    );
    if ret != IDOK.0 as isize {
        return; /* Canceled */
    }

    goto_data.i_line -= 1; /* Make it zero-based */

    /* Get ich (the target character index) from line number */
    let ich: i32 = if goto_data.i_line == 0 {
        0
    } else if goto_data.i_line >= goto_data.c_lines {
        cch
    } else {
        SendMessageW(
            g().h_edit,
            EM_LINEINDEX,
            Some(WPARAM(goto_data.i_line as usize)),
            Some(LPARAM(0)),
        ).0 as i32
    };

    /* EM_LINEINDEX can return -1 on failure */
    let ich = if ich < 0 { 0 } else { ich };

    /* Move the caret */
    SendMessageW(g().h_edit, EM_SETSEL, Some(WPARAM(ich as usize)), Some(LPARAM(ich as isize)));
    SendMessageW(g().h_edit, EM_SCROLLCARET, Some(WPARAM(0)), Some(LPARAM(0)));
}

pub unsafe fn status_bar_update_caret_pos() {
    let mut dw_start: u32 = 0;
    let mut dw_size: u32 = 0;

    SendMessageW(
        g().h_edit,
        EM_GETSEL,
        Some(WPARAM(&mut dw_start as *mut u32 as usize)),
        Some(LPARAM(&mut dw_size as *mut u32 as isize)),
    );
    let line = SendMessageW(g().h_edit, EM_LINEFROMCHAR, Some(WPARAM(dw_start as usize)), Some(LPARAM(0))).0 as i32;
    let ich = SendMessageW(g().h_edit, EM_LINEINDEX, Some(WPARAM(line as usize)), Some(LPARAM(0))).0 as i32;

    /* EM_LINEINDEX can return -1 on failure */
    let col = if ich < 0 { 0 } else { dw_start as i32 - ich };

    let fmt = crate::strings::to_string(&g().sz_status_bar_line_col);
    let buff = format_str(&fmt, &[Arg::I(line + 1), Arg::I(col + 1)]);
    SendMessageW(
        g().h_status_bar,
        SB_SETTEXTW,
        Some(WPARAM(SBPART_CURPOS)),
        Some(LPARAM(buff.as_ptr() as isize)),
    );
}

pub unsafe fn dialog_view_status_bar() {
    g().b_show_status_bar = !g().b_show_status_bar;
    do_show_hide_status_bar();
}

pub unsafe fn dialog_help_contents() {
    let helpfile = strings::wide("notepad.hlp");
    let _ = WinHelpW(
        Some(g().h_main_wnd),
        PCWSTR(helpfile.as_ptr()),
        HELP_INDEX as u32,
        0,
    );
}

pub unsafe fn dialog_help_about_notepad() {
    let sz_notepad = sw(STRING_NOTEPAD);
    let sz_notepad_authors = sw(STRING_NOTEPAD_AUTHORS);

    ShellAboutW(
        Some(g().h_main_wnd),
        PCWSTR(sz_notepad.as_ptr()),
        PCWSTR(sz_notepad_authors.as_ptr()),
        Some(g().h_icon),
    );
}


















