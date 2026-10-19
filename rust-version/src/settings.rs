//! Registry settings — port of settings.c (HKCU\Software\Microsoft\Notepad).

#![allow(non_snake_case)]

use crate::consts::*;
use crate::globals::g;
use crate::strings::sw;
use windows::Win32::Foundation::{ERROR_SUCCESS, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Globalization::GetUserDefaultLangID;
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, DeleteObject, GetDC, GetDeviceCaps, ReleaseDC, DEFAULT_CHARSET,
    DEFAULT_PITCH, FF_DONTCARE, FF_MODERN, FIXED_PITCH, FONT_CHARSET, FONT_CLIP_PRECISION,
    FONT_OUTPUT_PRECISION, FONT_QUALITY, FW_NORMAL, HGDIOBJ, LOGPIXELSY, LOGFONTW, HFONT,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegOpenKeyW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_SET_VALUE, REG_DWORD, REG_OPEN_CREATE_OPTIONS, REG_SZ,
};
use windows::Win32::System::WindowsProgramming::MulDiv;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, SendMessageW, SW_SHOWDEFAULT, SW_SHOWMAXIMIZED, WINDOWPLACEMENT,
    WINDOWPLACEMENT_FLAGS, WM_SETFONT, WPF_RESTORETOMAXIMIZED,
};
use windows::core::{w, PCWSTR};

const REGISTRY_KEY: PCWSTR = w!("Software\\Microsoft\\Notepad");
const LANG_CHINESE: u16 = 0x0004;
const LANG_JAPANESE: u16 = 0x0011;
const LANG_KOREAN: u16 = 0x0012;

unsafe fn height_from_point_size(dw_point_size: u32) -> i32 {
    let hdc = GetDC(None);
    let l_height = -MulDiv(dw_point_size as i32, GetDeviceCaps(Some(hdc), LOGPIXELSY), 720);
    ReleaseDC(None, hdc);
    l_height
}

unsafe fn point_size_from_height(l_height: i32) -> u32 {
    let hdc = GetDC(None);
    let mut dw_point_size = (-MulDiv(l_height, 720, GetDeviceCaps(Some(hdc), LOGPIXELSY))) as u32;
    ReleaseDC(None, hdc);

    /* round to nearest multiple of 10 */
    dw_point_size = dw_point_size.wrapping_add(5);
    dw_point_size -= dw_point_size % 10;

    dw_point_size
}

unsafe fn query_generic(
    h_key: HKEY,
    name: PCWSTR,
    expected: windows::Win32::System::Registry::REG_VALUE_TYPE,
    out: *mut u8,
    size: u32,
) -> bool {
    let mut temp = vec![0u8; size as usize];
    let mut dw_type = windows::Win32::System::Registry::REG_VALUE_TYPE(0);
    let mut cb_data = size;
    if RegQueryValueExW(
        h_key,
        name,
        None,
        Some(&mut dw_type),
        Some(temp.as_mut_ptr()),
        Some(&mut cb_data),
    ) != ERROR_SUCCESS
    {
        return false;
    }
    if dw_type != expected {
        return false;
    }
    std::ptr::copy_nonoverlapping(temp.as_ptr(), out, cb_data.min(size) as usize);
    true
}

unsafe fn query_dword(h_key: HKEY, name: PCWSTR) -> Option<u32> {
    let mut v = 0u32;
    if query_generic(h_key, name, REG_DWORD, &mut v as *mut u32 as *mut u8, 4) {
        Some(v)
    } else {
        None
    }
}

unsafe fn query_byte(h_key: HKEY, name: PCWSTR) -> Option<u8> {
    match query_dword(h_key, name) {
        Some(v) if v < 0x100 => Some(v as u8),
        _ => None,
    }
}

unsafe fn query_bool(h_key: HKEY, name: PCWSTR) -> Option<bool> {
    query_dword(h_key, name).map(|v| v != 0)
}

unsafe fn query_string(h_key: HKEY, name: PCWSTR, result: &mut [u16]) -> bool {
    if result.is_empty() {
        return false;
    }
    let size = (result.len() * 2) as u32;
    if !query_generic(h_key, name, REG_SZ, result.as_mut_ptr() as *mut u8, size) {
        return false;
    }
    result[result.len() - 1] = 0; /* Avoid buffer overrun */
    true
}

/// Copy a NUL-terminated wide string into a fixed buffer.
fn set_str(buf: &mut [u16], src: &[u16]) {
    let n = src.len().min(buf.len() - 1);
    buf[..n].copy_from_slice(&src[..n]);
    buf[n] = 0;
    for b in buf[n + 1..].iter_mut() {
        *b = 0;
    }
}

/// NOTEPAD_LoadSettingsFromRegistry — load settings from
/// HKCU\Software\Microsoft\Notepad and fill the WINDOWPLACEMENT.
pub unsafe fn load_settings_from_registry(p_wp: *mut WINDOWPLACEMENT) {
    let mut x = windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT as u32;
    let mut y = windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT as u32;
    let mut cx: u32 = 0;
    let mut cy: u32 = 0;
    let mut dw_point_size: u32 = 100;

    /* Set the default values */
    {
        let g = g();
        g.b_show_status_bar = true;
        g.b_wrap_long_lines = false;
        g.l_margins = RECT { left: 750, top: 1000, right: 750, bottom: 1000 };
        g.lf_font = LOGFONTW::default();
        g.lf_font.lfCharSet = DEFAULT_CHARSET;
        g.lf_font.lfWeight = FW_NORMAL.0 as i32;
        g.lf_font.lfPitchAndFamily = FIXED_PITCH.0 | FF_MODERN.0;
    }

    /* Open the target registry key */
    let mut h_key = HKEY::default();
    let have_key =
        RegOpenKeyW(HKEY_CURRENT_USER, REGISTRY_KEY, &mut h_key) == ERROR_SUCCESS;

    /* Load the values from registry */
    if have_key {
        if let Some(v) = query_byte(h_key, w!("lfCharSet")) {
            g().lf_font.lfCharSet = FONT_CHARSET(v);
        }
        if let Some(v) = query_byte(h_key, w!("lfClipPrecision")) {
            g().lf_font.lfClipPrecision = FONT_CLIP_PRECISION(v);
        }
        if let Some(v) = query_dword(h_key, w!("lfEscapement")) {
            g().lf_font.lfEscapement = v as i32;
        }
        if let Some(v) = query_byte(h_key, w!("lfItalic")) {
            g().lf_font.lfItalic = v;
        }
        if let Some(v) = query_dword(h_key, w!("lfOrientation")) {
            g().lf_font.lfOrientation = v as i32;
        }
        if let Some(v) = query_byte(h_key, w!("lfOutPrecision")) {
            g().lf_font.lfOutPrecision = FONT_OUTPUT_PRECISION(v);
        }
        if let Some(v) = query_byte(h_key, w!("lfPitchAndFamily")) {
            g().lf_font.lfPitchAndFamily = v;
        }
        if let Some(v) = query_byte(h_key, w!("lfQuality")) {
            g().lf_font.lfQuality = FONT_QUALITY(v);
        }
        if let Some(v) = query_byte(h_key, w!("lfStrikeOut")) {
            g().lf_font.lfStrikeOut = v;
        }
        if let Some(v) = query_byte(h_key, w!("lfUnderline")) {
            g().lf_font.lfUnderline = v;
        }
        if let Some(v) = query_dword(h_key, w!("lfWeight")) {
            g().lf_font.lfWeight = v as i32;
        }
        if let Some(v) = query_dword(h_key, w!("iPointSize")) {
            dw_point_size = v;
        }

        if let Some(v) = query_bool(h_key, w!("fWrap")) {
            g().b_wrap_long_lines = v;
        }
        if let Some(v) = query_bool(h_key, w!("fStatusBar")) {
            g().b_show_status_bar = v;
        }

        if let Some(v) = query_dword(h_key, w!("iMarginLeft")) {
            g().l_margins.left = v as i32;
        }
        if let Some(v) = query_dword(h_key, w!("iMarginTop")) {
            g().l_margins.top = v as i32;
        }
        if let Some(v) = query_dword(h_key, w!("iMarginRight")) {
            g().l_margins.right = v as i32;
        }
        if let Some(v) = query_dword(h_key, w!("iMarginBottom")) {
            g().l_margins.bottom = v as i32;
        }

        if let Some(v) = query_dword(h_key, w!("iWindowPosX")) {
            x = v;
        }
        if let Some(v) = query_dword(h_key, w!("iWindowPosY")) {
            y = v;
        }
        if let Some(v) = query_dword(h_key, w!("iWindowPosDX")) {
            cx = v;
        }
        if let Some(v) = query_dword(h_key, w!("iWindowPosDY")) {
            cy = v;
        }

        query_string(h_key, w!("searchString"), &mut g().sz_find_text);
        query_string(h_key, w!("replaceString"), &mut g().sz_replace_text);
    }

    /* Fill the WINDOWPLACEMENT */
    let wp = &mut *p_wp;
    wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
    wp.flags = WINDOWPLACEMENT_FLAGS(0);
    wp.showCmd = SW_SHOWDEFAULT.0 as u32;
    if cy & 0x8000_0000 != 0 {
        cy &= !0x8000_0000;
        wp.flags |= WPF_RESTORETOMAXIMIZED;
        wp.showCmd = SW_SHOWMAXIMIZED.0 as u32;
    }
    wp.rcNormalPosition = RECT {
        left: x as i32,
        top: y as i32,
        right: x.wrapping_add(cx) as i32,
        bottom: y.wrapping_add(cy) as i32,
    };
    wp.ptMaxPosition = POINT { x: x as i32, y: y as i32 };

    g().lf_font.lfHeight = height_from_point_size(dw_point_size);

    let mut ok = false;
    if have_key {
        ok = query_string(h_key, w!("lfFaceName"), &mut g().lf_font.lfFaceName);
    }
    if !ok {
        set_str(&mut g().lf_font.lfFaceName, &sw(STRING_DEFAULTFONT));
    }

    let mut ok = false;
    if have_key {
        ok = query_string(h_key, w!("szHeader"), &mut g().sz_header);
    }
    if !ok {
        set_str(&mut g().sz_header, &sw(STRING_PAGESETUP_HEADERVALUE));
    }

    let mut ok = false;
    if have_key {
        ok = query_string(h_key, w!("szTrailer"), &mut g().sz_footer);
    }
    if !ok {
        set_str(&mut g().sz_footer, &sw(STRING_PAGESETUP_FOOTERVALUE));
    }

    if have_key {
        let _ = RegCloseKey(h_key);
    }

    /* WORKAROUND: Far East Asian users may not have suitable fixed-pitch fonts. */
    match GetUserDefaultLangID() & 0x3FF {
        LANG_CHINESE | LANG_JAPANESE | LANG_KOREAN => {
            g().lf_font.lfPitchAndFamily = DEFAULT_PITCH.0 | FF_DONTCARE.0;
        }
        _ => {}
    }

    let h_font: HFONT = CreateFontIndirectW(&g().lf_font);
    let _ = SendMessageW(
        g().h_edit,
        WM_SETFONT,
        Some(WPARAM(h_font.0 as usize)),
        Some(LPARAM(1)),
    );
    if !h_font.is_invalid() {
        let g = g();
        if !g.h_font.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(g.h_font.0));
        }
        g.h_font = h_font;
    }
}

unsafe fn save_dword(h_key: HKEY, name: PCWSTR, value: u32) {
    let _ = RegSetValueExW(h_key, name, None, REG_DWORD, Some(&value.to_ne_bytes()));
}

unsafe fn save_string(h_key: HKEY, name: PCWSTR, value: &[u16]) {
    let len = value.iter().position(|&c| c == 0).unwrap_or(value.len());
    let bytes = std::slice::from_raw_parts(value.as_ptr() as *const u8, len * 2);
    let _ = RegSetValueExW(h_key, name, None, REG_SZ, Some(bytes));
}

/// NOTEPAD_SaveSettingsToRegistry — save settings to
/// HKCU\Software\Microsoft\Notepad.
pub unsafe fn save_settings_to_registry() {
    let mut wp = WINDOWPLACEMENT::default();
    wp.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
    let _ = GetWindowPlacement(g().h_main_wnd, &mut wp);
    let x = wp.rcNormalPosition.left as u32;
    let y = wp.rcNormalPosition.top as u32;
    let cx = wp
        .rcNormalPosition
        .right
        .wrapping_sub(wp.rcNormalPosition.left) as u32;
    let mut cy = wp
        .rcNormalPosition
        .bottom
        .wrapping_sub(wp.rcNormalPosition.top) as u32;
    if wp.flags.contains(WPF_RESTORETOMAXIMIZED) {
        cy |= 0x8000_0000;
    }

    let mut h_key = HKEY::default();
    if RegCreateKeyExW(
        HKEY_CURRENT_USER,
        REGISTRY_KEY,
        None,
        PCWSTR::null(),
        REG_OPEN_CREATE_OPTIONS(0),
        KEY_SET_VALUE,
        None,
        &mut h_key,
        None,
    ) == ERROR_SUCCESS
    {
        let g = g();
        save_dword(h_key, w!("lfCharSet"), g.lf_font.lfCharSet.0 as u32);
        save_dword(h_key, w!("lfClipPrecision"), g.lf_font.lfClipPrecision.0 as u32);
        save_dword(h_key, w!("lfEscapement"), g.lf_font.lfEscapement as u32);
        save_string(h_key, w!("lfFaceName"), &g.lf_font.lfFaceName);
        save_dword(h_key, w!("lfItalic"), g.lf_font.lfItalic as u32);
        save_dword(h_key, w!("lfOrientation"), g.lf_font.lfOrientation as u32);
        save_dword(h_key, w!("lfOutPrecision"), g.lf_font.lfOutPrecision.0 as u32);
        save_dword(h_key, w!("lfPitchAndFamily"), g.lf_font.lfPitchAndFamily as u32);
        save_dword(h_key, w!("lfQuality"), g.lf_font.lfQuality.0 as u32);
        save_dword(h_key, w!("lfStrikeOut"), g.lf_font.lfStrikeOut as u32);
        save_dword(h_key, w!("lfUnderline"), g.lf_font.lfUnderline as u32);
        save_dword(h_key, w!("lfWeight"), g.lf_font.lfWeight as u32);
        save_dword(
            h_key,
            w!("iPointSize"),
            point_size_from_height(g.lf_font.lfHeight),
        );
        save_dword(h_key, w!("fWrap"), g.b_wrap_long_lines as u32);
        save_dword(h_key, w!("fStatusBar"), g.b_show_status_bar as u32);
        save_string(h_key, w!("szHeader"), &g.sz_header);
        save_string(h_key, w!("szTrailer"), &g.sz_footer);
        save_dword(h_key, w!("iMarginLeft"), g.l_margins.left as u32);
        save_dword(h_key, w!("iMarginTop"), g.l_margins.top as u32);
        save_dword(h_key, w!("iMarginRight"), g.l_margins.right as u32);
        save_dword(h_key, w!("iMarginBottom"), g.l_margins.bottom as u32);
        save_dword(h_key, w!("iWindowPosX"), x);
        save_dword(h_key, w!("iWindowPosY"), y);
        save_dword(h_key, w!("iWindowPosDX"), cx);
        save_dword(h_key, w!("iWindowPosDY"), cy);
        save_string(h_key, w!("searchString"), &g.sz_find_text);
        save_string(h_key, w!("replaceString"), &g.sz_replace_text);

        let _ = RegCloseKey(h_key);
    }
}

