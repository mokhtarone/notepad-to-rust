//! printing.rs — port of printing.c (print dialog, printing thread,
//! page rendering with headers/footers, page setup dialog).

#![allow(non_snake_case)]
#![allow(static_mut_refs)]

use crate::consts::*;
use crate::globals::g;
use crate::strings::{sw, Arg};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, HANDLE, HLOCAL, LPARAM, RECT, SIZE, SYSTEMTIME, WPARAM,
};
use windows::Win32::Globalization::{GetDateFormatW, GetTimeFormatW, LOCALE_USER_DEFAULT};
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, DeleteDC, DeleteObject, DrawTextW, GetDeviceCaps, GetStockObject,
    GetTextExtentPoint32W, GetTextMetricsW, InflateRect, Rectangle, SelectObject, SetMapMode,
    TextOutW, BLACK_PEN, DT_CENTER, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER,
    FW_BOLD, HFONT, HGDIOBJ, HORZRES, LOGPIXELSX, LOGPIXELSY, MM_TEXT, NULL_BRUSH,
    PHYSICALHEIGHT, PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH, TEXTMETRICW, VERTRES,
};
use windows::Win32::Storage::Xps::{AbortDoc, EndDoc, EndPage, StartDocW, StartPage, DOCINFOW};
use windows::Win32::System::Memory::{LocalLock, LocalUnlock};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Threading::{CreateThread, THREAD_CREATION_FLAGS};
use windows::Win32::System::WindowsProgramming::MulDiv;
use windows::Win32::UI::Controls::{EM_GETHANDLE, EM_GETSEL};
use windows::Win32::UI::Controls::Dialogs::{
    PageSetupDlgW, PrintDlgW, PAGESETUPDLGW, PD_NOSELECTION, PD_PAGENUMS, PD_RETURNDC,
    PD_SELECTION, PRINTDLGW, PSD_ENABLEPAGESETUPHOOK, PSD_ENABLEPAGESETUPTEMPLATE, PSD_MARGINS,
};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, DialogBoxIndirectParamW, DLGPROC, DLGTEMPLATE, EndDialog, GetDlgItem,
    GetDlgItemTextW, GetWindowTextLengthW, GetWindowTextW, IDABORT, IDCANCEL, IDHELP, IDOK,
    MB_ICONEXCLAMATION, MB_OK, MessageBoxW, PostMessageW, SendMessageW, SetDlgItemTextW,
    WM_COMMAND, WM_DESTROY, WM_INITDIALOG,
};
use windows::Win32::Foundation::HWND;

/* Convert the points into pixels */
#[inline]
fn x_points_to_pixels(hdc: HDC, points: i32) -> i32 {
    unsafe { MulDiv(points, GetDeviceCaps(Some(hdc), LOGPIXELSX), 72) }
}

#[inline]
fn y_points_to_pixels(hdc: HDC, points: i32) -> i32 {
    unsafe { MulDiv(points, GetDeviceCaps(Some(hdc), LOGPIXELSY), 72) }
}

use windows::Win32::Graphics::Gdi::HDC;

const BODY_FONT_SIZE: i32 = 10; /* 10pt */
const HEADER_FONT_SIZE: i32 = 9; /* 9pt */
const SPACING_HEIGHT: i32 = 4; /* 4pt */

struct PrintData {
    printer: PRINTDLGW,
    hwnd_dlg: HWND,
    status: u32, // string resource id
    current_page: u32,
    print_rect: RECT,
    st_now: SYSTEMTIME,
    h_header_font: HFONT,
    h_body_font: HFONT,
    psz_text: Vec<u16>, // the text to print (C: HeapAlloc'ed buffer)
    ich: usize,
    cch_text: usize,
    cy_header: i32,
    cy_spacing: i32,
    cy_footer: i32,
}

unsafe fn alert_print_error() {
    let sz_untitled = sw(STRING_UNTITLED);

    let name: &[u16] = if g().sz_file_name[0] != 0 {
        &g().sz_file_name
    } else {
        &sz_untitled
    };

    crate::dialogs::string_msg_box(
        Some(g().h_main_wnd),
        STRING_PRINTERROR,
        name,
        MB_ICONEXCLAMATION | MB_OK,
    );
}

unsafe fn get_printing_rect(hdc: HDC, p_margins: &RECT) -> RECT {
    let i_log_pixels_x = GetDeviceCaps(Some(hdc), LOGPIXELSX);
    let i_log_pixels_y = GetDeviceCaps(Some(hdc), LOGPIXELSY);
    let i_horz_res = GetDeviceCaps(Some(hdc), HORZRES); /* in pixels */
    let i_vert_res = GetDeviceCaps(Some(hdc), VERTRES); /* in pixels */

    let convert_x = |x: i32| MulDiv(x, i_log_pixels_x, 2540); /* 100th mm to pixels */
    let convert_y = |y: i32| MulDiv(y, i_log_pixels_y, 2540);

    let mut rc_print_rect = RECT {
        left: convert_x(p_margins.left),
        top: convert_y(p_margins.top),
        right: i_horz_res - convert_x(p_margins.right),
        bottom: i_vert_res - convert_y(p_margins.bottom),
    };

    let mut rc_physical = RECT {
        left: GetDeviceCaps(Some(hdc), PHYSICALOFFSETX),
        top: GetDeviceCaps(Some(hdc), PHYSICALOFFSETY),
        right: 0,
        bottom: 0,
    };
    rc_physical.right = rc_physical.left + GetDeviceCaps(Some(hdc), PHYSICALWIDTH);
    rc_physical.bottom = rc_physical.top + GetDeviceCaps(Some(hdc), PHYSICALHEIGHT);

    /* Adjust the margin */
    rc_print_rect.left = rc_print_rect.left.max(rc_physical.left);
    rc_print_rect.top = rc_print_rect.top.max(rc_physical.top);
    rc_print_rect.right = rc_print_rect.right.min(rc_physical.right);
    rc_print_rect.bottom = rc_print_rect.bottom.min(rc_physical.bottom);

    rc_print_rect
}

/* Returns the length of the selection in characters */
unsafe fn get_selection_text_length(hwnd: HWND) -> i32 {
    let mut dw_start: u32 = 0;
    let mut dw_end: u32 = 0;
    SendMessageW(
        hwnd,
        EM_GETSEL,
        Some(WPARAM(&mut dw_start as *mut u32 as usize)),
        Some(LPARAM(&mut dw_end as *mut u32 as isize)),
    );
    dw_end.wrapping_sub(dw_start) as i32
}

/* Copies the selection into lp_string; n_max_count includes the NUL terminator */
unsafe fn get_selection_text(hwnd: HWND, lp_string: &mut [u16], n_max_count: i32) -> i32 {
    let mut dw_start: u32 = 0;
    let mut dw_end: u32 = 0;
    let cch_text = GetWindowTextLengthW(hwnd);

    SendMessageW(
        hwnd,
        EM_GETSEL,
        Some(WPARAM(&mut dw_start as *mut u32 as usize)),
        Some(LPARAM(&mut dw_end as *mut u32 as isize)),
    );
    if dw_start == dw_end || cch_text == 0 {
        return 0;
    }

    let h_local = HLOCAL(
        SendMessageW(hwnd, EM_GETHANDLE, Some(WPARAM(0)), Some(LPARAM(0))).0
            as *mut core::ffi::c_void,
    );
    let psz_text = LocalLock(h_local) as *const u16;
    if psz_text.is_null() {
        return 0;
    }

    let sel_len = (dw_end - dw_start) as usize;
    let max_chars = n_max_count.max(0) as usize;
    let copy_len = sel_len.min(max_chars.saturating_sub(1));
    let src = std::slice::from_raw_parts(psz_text.add(dw_start as usize), sel_len);
    lp_string[..copy_len].copy_from_slice(&src[..copy_len]);
    if copy_len < lp_string.len() {
        lp_string[copy_len] = 0;
    }
    let _ = LocalUnlock(h_local);

    if copy_len == sel_len {
        sel_len as i32
    } else {
        n_max_count - 1 /* STRSAFE_E_INSUFFICIENT_BUFFER */
    }
}

/*
 * See also:
 * https://support.microsoft.com/en-us/windows/help-in-notepad-4d68c388-2ff2-0e7f-b706-35fb2ab88a8c
 */
unsafe fn draw_header_or_footer(
    hdc: HDC,
    p_rect: &mut RECT,
    psz_format: &[u16],
    n_page_no: i32,
    pst_now: &SYSTEMTIME,
) {
    let mut sz_text: Vec<u16> = Vec::with_capacity(256);
    let mut sz_field = [0u16; 128];
    let mut u_align = DT_CENTER;
    let u_flags = DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX;

    /* Draw a rectangle */
    let h_old_pen = SelectObject(hdc, GetStockObject(BLACK_PEN));
    let h_old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
    let _ = Rectangle(hdc, p_rect.left, p_rect.top, p_rect.right, p_rect.bottom);
    SelectObject(hdc, h_old_brush);
    SelectObject(hdc, h_old_pen);

    let _ = InflateRect(p_rect, -x_points_to_pixels(hdc, 3), 0); /* Shrink 3pt */

    let mut fmt = psz_format.iter().copied().take_while(|&c| c != 0);
    while let Some(ch) = fmt.next() {
        if ch != 0x26 /* '&' */ {
            sz_text.push(ch);
            continue;
        }

        let ch = match fmt.next() {
            Some(c) => c,
            None => break,
        };

        /* Make it uppercase (ASCII) */
        let ch_upper = if (0x61..=0x7A).contains(&ch) {
            ch - 0x20
        } else {
            ch
        };
        let c = char::from_u32(ch_upper as u32).unwrap_or('\0');

        match c {
            '&' => {
                /* Found double ampersand */
                sz_text.push(0x26);
            }
            'L' => {
                /* Left */
                DrawTextW(hdc, sz_text.as_mut_slice(), p_rect, u_align | u_flags);
                sz_text.clear();
                u_align = DT_LEFT;
            }
            'C' => {
                /* Center */
                DrawTextW(hdc, sz_text.as_mut_slice(), p_rect, u_align | u_flags);
                sz_text.clear();
                u_align = DT_CENTER;
            }
            'R' => {
                /* Right */
                DrawTextW(hdc, sz_text.as_mut_slice(), p_rect, u_align | u_flags);
                sz_text.clear();
                u_align = DT_RIGHT;
            }
            'D' => {
                /* Date */
                GetDateFormatW(
                    LOCALE_USER_DEFAULT,
                    0,
                    Some(pst_now),
                    PCWSTR::null(),
                    Some(&mut sz_field),
                );
                sz_text.extend(sz_field.iter().take_while(|&&c| c != 0));
            }
            'T' => {
                /* Time */
                GetTimeFormatW(
                    LOCALE_USER_DEFAULT,
                    0,
                    Some(pst_now),
                    PCWSTR::null(),
                    Some(&mut sz_field),
                );
                sz_text.extend(sz_field.iter().take_while(|&&c| c != 0));
            }
            'F' => {
                /* Filename */
                sz_text.extend(g().sz_file_title.iter().take_while(|&&c| c != 0));
            }
            'P' => {
                /* Page number */
                sz_text.extend(n_page_no.to_string().encode_utf16());
            }
            _ => {
                /* Otherwise */
                sz_text.push(0x26);
                sz_text.push(ch);
            }
        }
    }

    DrawTextW(hdc, sz_text.as_mut_slice(), p_rect, u_align | u_flags);
}

/* Flush the accumulated text run; returns TRUE if printing is being canceled */
unsafe fn flush_text(
    data: &PrintData,
    hdc: HDC,
    b_skip_page: bool,
    y_top: i32,
    ich_start: &mut usize,
    x_start: &mut i32,
    x_left: i32,
) -> bool {
    if *ich_start < data.ich && !b_skip_page {
        let _ = TextOutW(hdc, *x_start, y_top, &data.psz_text[*ich_start..data.ich]);
    }
    *ich_start = data.ich;
    *x_start = x_left;
    data.status == STRING_PRINTCANCELING
}

unsafe fn do_print_body(data: &mut PrintData, _page_count: u32, b_skip_page: bool) -> bool {
    let hdc = data.printer.hDC;
    let print_rect = data.print_rect;
    let mut x_left = print_rect.left;
    let mut y_top = print_rect.top + data.cy_header + data.cy_spacing;
    let mut x_start;
    let mut ich_start;
    let mut char_metrics = SIZE::default();
    let mut tm_text = TEXTMETRICW::default();

    /* Calculate a tab width */
    const TAB_STOP: i32 = 8;
    let _ = GetTextMetricsW(hdc, &mut tm_text);
    let tab_width = TAB_STOP * tm_text.tmAveCharWidth;

    /* The drawing-body loop */
    ich_start = data.ich;
    x_start = x_left;
    while data.ich < data.cch_text {
        let ch = data.psz_text[data.ich];

        if ch == 0x0D /* '\r' */ {
            if flush_text(data, hdc, b_skip_page, y_top, &mut ich_start, &mut x_start, x_left) {
                return false;
            }

            data.ich += 1; /* Next char */
            ich_start = data.ich;
            continue;
        }

        if ch == 0x0A /* '\n' */ {
            if flush_text(data, hdc, b_skip_page, y_top, &mut ich_start, &mut x_start, x_left) {
                return false;
            }

            /* Next line */
            y_top += tm_text.tmHeight;
            x_left = print_rect.left;
            x_start = x_left;
        } else {
            if ch == 0x09 /* '\t' */ {
                let n_step_width = tab_width - ((x_left - print_rect.left) % tab_width);

                if flush_text(data, hdc, b_skip_page, y_top, &mut ich_start, &mut x_start, x_left) {
                    return false;
                }

                /* Go to the next tab stop */
                x_left += n_step_width;
                x_start = x_left;
            } else {
                /* Normal char */
                let _ = GetTextExtentPoint32W(hdc, &[ch], &mut char_metrics);
                x_left += char_metrics.cx;
            }

            /* Insert a line break if the next position reached the right edge */
            if x_left + char_metrics.cx >= print_rect.right {
                if ch != 0x09 {
                    if flush_text(data, hdc, b_skip_page, y_top, &mut ich_start, &mut x_start, x_left) {
                        return false;
                    }
                }

                /* Next line */
                y_top += tm_text.tmHeight;
                x_left = print_rect.left;
                x_start = x_left;
            }
        }

        data.ich += 1; /* Next char */
        if ch == 0x09 || ch == 0x0A {
            ich_start = data.ich;
        }

        if y_top + tm_text.tmHeight >= print_rect.bottom - data.cy_footer {
            break; /* The next line reached the body bottom */
        }
    }

    if flush_text(data, hdc, b_skip_page, y_top, &mut ich_start, &mut x_start, x_left) {
        return false;
    }
    true
}

unsafe fn do_print_page(data: &mut PrintData, page_count: u32) -> bool {
    let hdc = data.printer.hDC;

    /* Should we skip this page? */
    let b_skip_page = (data.printer.Flags & PD_SELECTION).0 == 0
        && (data.printer.Flags & PD_PAGENUMS).0 != 0
        && !((data.printer.nFromPage as u32 <= page_count)
            && (page_count <= data.printer.nToPage as u32));

    /* The prologue of a page */
    if !b_skip_page {
        if StartPage(hdc) <= 0 {
            data.status = STRING_PRINTFAILED;
            return false;
        }

        if data.cy_header > 0 {
            /* Draw the page header */
            let mut rc = data.print_rect;
            rc.bottom = rc.top + data.cy_header;

            let h_old_font = SelectObject(hdc, HGDIOBJ(data.h_header_font.0));
            draw_header_or_footer(hdc, &mut rc, &g().sz_header, page_count as i32, &data.st_now);
            SelectObject(hdc, h_old_font); /* De-select the font */
        }
    }

    let h_old_font = SelectObject(hdc, HGDIOBJ(data.h_body_font.0));
    let ret = do_print_body(data, page_count, b_skip_page);
    SelectObject(hdc, h_old_font);
    if !ret {
        return false; /* Canceled */
    }

    /* The epilogue of a page */
    if !b_skip_page {
        if data.cy_footer > 0 {
            /* Draw the page footer */
            let mut rc = data.print_rect;
            rc.top = rc.bottom - data.cy_footer;

            let h_old_font = SelectObject(hdc, HGDIOBJ(data.h_header_font.0));
            draw_header_or_footer(hdc, &mut rc, &g().sz_footer, page_count as i32, &data.st_now);
            SelectObject(hdc, h_old_font);
        }

        if EndPage(hdc) <= 0 {
            data.status = STRING_PRINTFAILED;
            return false;
        }
    }

    true
}

unsafe fn do_create_print_fonts(data: &mut PrintData) -> bool {
    let hdc = data.printer.hDC;

    /* Create the main text font for printing */
    let mut lf_body = g().lf_font;
    lf_body.lfHeight = -y_points_to_pixels(hdc, BODY_FONT_SIZE);
    data.h_body_font = CreateFontIndirectW(&lf_body);
    if data.h_body_font.is_invalid() {
        return false;
    }

    /* Create the header/footer font */
    let mut lf_header = g().lf_font;
    lf_header.lfHeight = -y_points_to_pixels(hdc, HEADER_FONT_SIZE);
    lf_header.lfWeight = FW_BOLD.0 as i32;
    data.h_header_font = CreateFontIndirectW(&lf_header);
    if data.h_header_font.is_invalid() {
        return false;
    }

    true
}

/* The core of DoPrintDocument before the common cleanup */
unsafe fn do_print_document_core(data: &mut PrintData) -> bool {
    let hdc = data.printer.hDC;

    data.st_now = GetLocalTime();

    data.print_rect = get_printing_rect(hdc, &g().l_margins);

    if !do_create_print_fonts(data) {
        data.status = STRING_PRINTFAILED;
        return false;
    }

    if (data.printer.Flags & PD_SELECTION).0 != 0 {
        data.cch_text = get_selection_text_length(g().h_edit) as usize;
    } else {
        data.cch_text = GetWindowTextLengthW(g().h_edit) as usize;
    }

    /* Allocate a buffer for the text */
    data.psz_text = vec![0u16; data.cch_text + 1];

    if (data.printer.Flags & PD_SELECTION).0 != 0 {
        let mut buf = std::mem::take(&mut data.psz_text);
        get_selection_text(g().h_edit, &mut buf, data.cch_text as i32 + 1);
        data.psz_text = buf;
    } else {
        let mut buf = std::mem::take(&mut data.psz_text);
        GetWindowTextW(g().h_edit, &mut buf);
        data.psz_text = buf;
    }
    data.psz_text.truncate(data.cch_text);

    /* Start a document */
    let doc_name = g().sz_file_title.to_vec();
    let doc_info = DOCINFOW {
        cbSize: std::mem::size_of::<DOCINFOW>() as i32,
        lpszDocName: PCWSTR(doc_name.as_ptr()),
        lpszOutput: PCWSTR::null(),
        lpszDatatype: PCWSTR::null(),
        fwType: 0,
    };
    if StartDocW(hdc, &doc_info) <= 0 {
        data.status = STRING_PRINTFAILED;
        return false;
    }

    /* Calculate the header and footer heights */
    let mut tm_header = TEXTMETRICW::default();
    let h_old_font = SelectObject(hdc, HGDIOBJ(data.h_header_font.0));
    let _ = GetTextMetricsW(hdc, &mut tm_header);
    data.cy_header = 2 * tm_header.tmHeight;
    data.cy_footer = data.cy_header;
    data.cy_spacing = y_points_to_pixels(hdc, SPACING_HEIGHT);
    SelectObject(hdc, h_old_font); /* De-select the font */
    if g().sz_header[0] == 0 {
        data.cy_header = 0;
        data.cy_spacing = 0;
    }
    if g().sz_footer[0] == 0 {
        data.cy_footer = 0;
    }

    /* The printing-copies loop */
    let n_copies = data.printer.nCopies as u32;
    for _copy_count in 1..=n_copies {
        /* The printing-pages loop */
        data.ich = 0;
        let mut page_count: u32 = 1;
        while data.ich < data.cch_text {
            data.current_page = page_count;
            let _ = PostMessageW(Some(data.hwnd_dlg), PRINTING_MESSAGE, WPARAM(0), LPARAM(0));

            if !do_print_page(data, page_count) {
                AbortDoc(hdc); /* Cancel printing */
                return false;
            }
            page_count += 1;
        }
    }

    if EndDoc(hdc) <= 0 {
        data.status = STRING_PRINTFAILED;
        return false;
    }

    data.status = STRING_PRINTCOMPLETE;
    true
}

unsafe fn do_print_document(data: &mut PrintData) -> bool {
    let ret = do_print_document_core(data);

    /* Quit: common cleanup (mirrors the C "goto Quit" path) */
    let _ = DeleteObject(HGDIOBJ(data.h_header_font.0));
    let _ = DeleteObject(HGDIOBJ(data.h_body_font.0));
    data.psz_text = Vec::new();
    if data.status == STRING_PRINTCANCELING {
        data.status = STRING_PRINTCANCELED;
    }
    let _ = PostMessageW(Some(data.hwnd_dlg), PRINTING_MESSAGE, WPARAM(0), LPARAM(0));
    ret
}

unsafe extern "system" fn print_thread_func(arg: *mut core::ffi::c_void) -> u32 {
    let data = &mut *(arg as *mut PrintData);
    data.current_page = 1;
    data.status = STRING_NOWPRINTING;
    let _ = PostMessageW(Some(data.hwnd_dlg), PRINTING_MESSAGE, WPARAM(0), LPARAM(0));
    do_print_document(data) as u32
}

static mut S_P_DATA: Option<*mut PrintData> = None;
static mut S_H_THREAD: Option<HANDLE> = None;
static mut S_SZ_PAGE: [u16; 64] = [0; 64];

unsafe extern "system" fn dialog_printing_proc(
    hwnd: HWND,
    u_msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> isize {
    match u_msg {
        WM_INITDIALOG => {
            S_P_DATA = Some(l_param.0 as *mut PrintData);
            let data = &mut *S_P_DATA.unwrap();
            data.hwnd_dlg = hwnd;
            let _ = SetDlgItemTextW(
                hwnd,
                IDC_PRINTING_FILENAME as i32,
                PCWSTR(g().sz_file_title.as_ptr()),
            );
            GetDlgItemTextW(hwnd, IDC_PRINTING_PAGE as i32, &mut S_SZ_PAGE);
            let _ = SetDlgItemTextW(hwnd, IDC_PRINTING_PAGE as i32, PCWSTR::null());

            match CreateThread(
                None,
                0,
                Some(print_thread_func),
                Some(S_P_DATA.unwrap() as *mut core::ffi::c_void),
                THREAD_CREATION_FLAGS(0),
                None,
            ) {
                Ok(h) => S_H_THREAD = Some(h),
                Err(_) => {
                    data.status = STRING_PRINTFAILED;
                    let _ = EndDialog(hwnd, IDABORT.0 as isize);
                }
            }
            1 /* TRUE */
        }

        PRINTING_MESSAGE => {
            if let Some(p_data) = S_P_DATA {
                let data = &mut *p_data;
                match data.status {
                    STRING_NOWPRINTING | STRING_PRINTCANCELING => {
                        let fmt = crate::strings::to_string(&S_SZ_PAGE);
                        let sz_text =
                            crate::strings::format_str(&fmt, &[Arg::U(data.current_page)]);
                        let _ = SetDlgItemTextW(
                            hwnd,
                            IDC_PRINTING_PAGE as i32,
                            PCWSTR(sz_text.as_ptr()),
                        );

                        let sz_status = sw(data.status);
                        let _ = SetDlgItemTextW(
                            hwnd,
                            IDC_PRINTING_STATUS as i32,
                            PCWSTR(sz_status.as_ptr()),
                        );
                    }

                    STRING_PRINTCOMPLETE | STRING_PRINTCANCELED | STRING_PRINTFAILED => {
                        let sz_status = sw(data.status);
                        let _ = SetDlgItemTextW(
                            hwnd,
                            IDC_PRINTING_STATUS as i32,
                            PCWSTR(sz_status.as_ptr()),
                        );

                        if data.status == STRING_PRINTCOMPLETE {
                            let _ = EndDialog(hwnd, IDOK.0 as isize);
                        } else if data.status == STRING_PRINTFAILED {
                            let _ = EndDialog(hwnd, IDABORT.0 as isize);
                        } else {
                            let _ = EndDialog(hwnd, IDCANCEL.0 as isize);
                        }
                    }

                    _ => {}
                }
            }
            0
        }

        WM_COMMAND => {
            if (w_param.0 & 0xFFFF) == IDCANCEL.0 as usize {
                if let Some(p_data) = S_P_DATA {
                    let data = &mut *p_data;
                    if data.status == STRING_NOWPRINTING {
                        if let Ok(h_dlg) = GetDlgItem(Some(hwnd), IDCANCEL.0 as i32) {
                            let _ = EnableWindow(h_dlg, false);
                        }
                        data.status = STRING_PRINTCANCELING;
                        let _ =
                            PostMessageW(Some(hwnd), PRINTING_MESSAGE, WPARAM(0), LPARAM(0));
                    }
                }
            }
            0
        }

        WM_DESTROY => {
            if let Some(h_thread) = S_H_THREAD {
                let _ = CloseHandle(h_thread);
                S_H_THREAD = None;
            }
            if let Some(p_data) = S_P_DATA {
                let data = Box::from_raw(p_data);
                let _ = DeleteDC(data.printer.hDC);
                drop(data);
                S_P_DATA = None;
            }
            0
        }

        _ => 0,
    }
}

pub unsafe fn dialog_file_print() {
    let mut print_data = Box::new(PrintData {
        printer: PRINTDLGW::default(),
        hwnd_dlg: HWND::default(),
        status: 0,
        current_page: 0,
        print_rect: RECT::default(),
        st_now: SYSTEMTIME::default(),
        h_header_font: HFONT::default(),
        h_body_font: HFONT::default(),
        psz_text: Vec::new(),
        ich: 0,
        cch_text: 0,
        cy_header: 0,
        cy_spacing: 0,
        cy_footer: 0,
    });

    let printer = &mut print_data.printer;
    printer.lStructSize = std::mem::size_of::<PRINTDLGW>() as u32;
    printer.hwndOwner = g().h_main_wnd;
    printer.Flags = PD_RETURNDC | PD_SELECTION;

    /* Disable the selection radio button if there is no text selected */
    if get_selection_text_length(g().h_edit) == 0 {
        printer.Flags |= PD_NOSELECTION;
    }

    printer.nFromPage = 1;
    printer.nToPage = 0xFFFF; /* MAXWORD */
    printer.nMinPage = 1;
    printer.nMaxPage = 0xFFFF; /* MAXWORD */

    printer.hDevMode = g().h_dev_mode;
    printer.hDevNames = g().h_dev_names;

    let p_data = Box::into_raw(print_data);
    let printer = &mut (*p_data).printer;

    let ret = PrintDlgW(printer);
    /* NOTE: Even if PrintDlg returns FALSE, hDevMode and hDevNames may have changed. */
    g().h_dev_mode = printer.hDevMode;
    g().h_dev_names = printer.hDevNames;

    if !ret.as_bool() {
        drop(Box::from_raw(p_data));
        return; /* The user canceled printing */
    }

    /* Ensure that each logical unit maps to one pixel */
    let _ = SetMapMode(printer.hDC, MM_TEXT);

    let template = crate::resources::printing_dialog_template();
    /* Copy into a u32-aligned buffer: DLGTEMPLATE requires DWORD alignment. */
    let mut aligned = vec![0u32; template.len().div_ceil(4)];
    let bytes =
        std::slice::from_raw_parts_mut(aligned.as_mut_ptr() as *mut u8, aligned.len() * 4);
    bytes[..template.len()].copy_from_slice(&template);

    let dlg_ret = DialogBoxIndirectParamW(
        Some(g().h_instance),
        aligned.as_ptr() as *const DLGTEMPLATE,
        Some(g().h_main_wnd),
        {
            let proc: DLGPROC = Some(dialog_printing_proc);
            proc
        },
        LPARAM(p_data as isize),
    );
    if dlg_ret == IDABORT.0 as isize {
        alert_print_error();
    }
}

/*
 *           DIALOG_PAGESETUP_Hook
 */
unsafe extern "system" fn dialog_page_setup_hook(
    h_dlg: HWND,
    u_msg: u32,
    w_param: WPARAM,
    _l_param: LPARAM,
) -> usize {
    match u_msg {
        WM_INITDIALOG => {
            /* fetch last user input prior to display dialog */
            let _ = SetDlgItemTextW(h_dlg, 0x141, PCWSTR(g().sz_header.as_ptr()));
            let _ = SetDlgItemTextW(h_dlg, 0x143, PCWSTR(g().sz_footer.as_ptr()));
            0 /* FALSE */
        }

        WM_COMMAND => {
            if ((w_param.0 >> 16) & 0xFFFF) as u32 == BN_CLICKED {
                let cmd = (w_param.0 & 0xFFFF) as u32;
                if cmd == IDOK.0 as u32 {
                    /* save user input and close dialog */
                    GetDlgItemTextW(h_dlg, 0x141, &mut g().sz_header);
                    GetDlgItemTextW(h_dlg, 0x143, &mut g().sz_footer);
                    return 0; /* FALSE */
                } else if cmd == IDCANCEL.0 as u32 {
                    /* discard user input and close dialog */
                    return 0; /* FALSE */
                } else if cmd == IDHELP.0 as u32 {
                    /* FIXME: Bring this to work */
                    let sorry = crate::strings::wide("Sorry, no help available");
                    let help = crate::strings::wide("Help");
                    MessageBoxW(
                        Some(g().h_main_wnd),
                        PCWSTR(sorry.as_ptr()),
                        PCWSTR(help.as_ptr()),
                        MB_ICONEXCLAMATION,
                    );
                    return 1; /* TRUE */
                }
            }
            0 /* FALSE */
        }

        _ => 0, /* FALSE */
    }
}


/*           DIALOG_FilePageSetup
 */
pub unsafe fn dialog_file_page_setup() {
    let mut page = PAGESETUPDLGW {
        lStructSize: std::mem::size_of::<PAGESETUPDLGW>() as u32,
        hwndOwner: g().h_main_wnd,
        Flags: PSD_ENABLEPAGESETUPTEMPLATE | PSD_ENABLEPAGESETUPHOOK | PSD_MARGINS,
        hInstance: g().h_instance,
        rtMargin: g().l_margins,
        hDevMode: g().h_dev_mode,
        hDevNames: g().h_dev_names,
        lpPageSetupTemplateName: PCWSTR(DIALOG_PAGESETUP as usize as *const u16),
        lpfnPageSetupHook: Some(dialog_page_setup_hook),
        ..Default::default()
    };

    let _ = PageSetupDlgW(&mut page);

    /* NOTE: Even if PageSetupDlg returns FALSE, the following members may have changed */
    g().h_dev_mode = page.hDevMode;
    g().h_dev_names = page.hDevNames;
    g().l_margins = page.rtMargin;
}
