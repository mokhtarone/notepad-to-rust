//! ReadText / WriteText — port of text.c (encoding analysis, BOM handling,
//! EOLN normalization, NUL -> SPACE conversion).

#![allow(non_upper_case_globals)]

use crate::consts::{Encoding, Eoln, ENCODING_DEFAULT};
use windows::Win32::Foundation::{GetLastError, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Globalization::{
    IsTextUnicode, IS_TEXT_UNICODE_RESULT, MultiByteToWideChar, MULTI_BYTE_TO_WIDE_CHAR_FLAGS,
    WideCharToMultiByte, CP_UTF8, MB_ERR_INVALID_CHARS,
};
use windows::Win32::Storage::FileSystem::{GetFileSize, INVALID_FILE_SIZE};
use windows::Win32::System::Memory::{
    CreateFileMappingW, LocalAlloc, LocalLock, LocalUnlock, MapViewOfFile, UnmapViewOfFile,
    FILE_MAP_READ, LMEM_MOVEABLE, PAGE_READONLY,
};

const CP_ACP: u32 = 0;
const IS_TEXT_UNICODE_STATISTICS: i32 = 0x0002;
const IS_TEXT_UNICODE_REVERSE_STATISTICS: i32 = 0x0008;
const IS_TEXT_UNICODE_REVERSE_MASK: i32 = 0x00F0;
const MAXLONG: i32 = 0x7FFF_FFFF;

fn is_text_non_zero_ascii(bytes: &[u8]) -> bool {
    bytes.iter().all(|&b| b != 0 && b < 0x80)
}

/// AnalyzeEncoding — guess the encoding of a BOM-less byte stream.
unsafe fn analyze_encoding(bytes: &[u8]) -> Encoding {
    let mut flags: i32 = IS_TEXT_UNICODE_STATISTICS | IS_TEXT_UNICODE_REVERSE_STATISTICS;

    if is_text_non_zero_ascii(bytes) {
        return ENCODING_DEFAULT;
    }

    let mut result = IS_TEXT_UNICODE_RESULT(flags as u32);
    let is_unicode =
        IsTextUnicode(bytes.as_ptr() as *const _, bytes.len() as i32, Some(&mut result)).as_bool();
    flags = result.0 as i32;
    if is_unicode {
        return Encoding::Utf16Le;
    }

    if (flags & IS_TEXT_UNICODE_REVERSE_MASK) == IS_TEXT_UNICODE_REVERSE_STATISTICS {
        return Encoding::Utf16Be;
    }

    // is it UTF-8?
    if multi_to_wide(CP_UTF8, MB_ERR_INVALID_CHARS, bytes).is_some() {
        return Encoding::Utf8;
    }

    Encoding::Ansi
}

/// MultiByteToWideChar wrapper returning an owned wide buffer.
unsafe fn multi_to_wide(
    codepage: u32,
    flags: MULTI_BYTE_TO_WIDE_CHAR_FLAGS,
    bytes: &[u8],
) -> Option<Vec<u16>> {
    if bytes.is_empty() {
        return Some(Vec::new());
    }
    let cch = MultiByteToWideChar(codepage, flags, bytes, None);
    if cch <= 0 {
        return None;
    }
    let mut buf = vec![0u16; cch as usize];
    let written = MultiByteToWideChar(codepage, flags, bytes, Some(&mut buf));
    if written <= 0 {
        return None;
    }
    buf.truncate(written as usize);
    Some(buf)
}

/// WideCharToMultiByte wrapper returning an owned byte buffer.
unsafe fn wide_to_multi(codepage: u32, text: &[u16]) -> Option<Vec<u8>> {
    let cch = WideCharToMultiByte(codepage, 0, text, None, windows::core::PCSTR::null(), None);
    if cch <= 0 {
        return None;
    }
    let mut buf = vec![0u8; cch as usize];
    let written =
        WideCharToMultiByte(codepage, 0, text, Some(&mut buf), windows::core::PCSTR::null(), None);
    if written <= 0 {
        return None;
    }
    buf.truncate(written as usize);
    Some(buf)
}

/// ReplaceNewLines — convert every LF / CR into CRLF in the new buffer.
unsafe fn replace_new_lines(psz_new: &mut [u16], psz_old: &[u16]) {
    let mut b_prev_cr = false;
    let mut ich_new = 0usize;

    for &ch in psz_old.iter() {
        if ch == '\n' as u16 {
            if !b_prev_cr {
                psz_new[ich_new] = '\r' as u16;
                ich_new += 1;
                psz_new[ich_new] = '\n' as u16;
                ich_new += 1;
            }
        } else if ch == '\r' as u16 {
            psz_new[ich_new] = '\r' as u16;
            ich_new += 1;
            psz_new[ich_new] = '\n' as u16;
            ich_new += 1;
        } else {
            psz_new[ich_new] = ch;
            ich_new += 1;
        }
        b_prev_cr = ch == '\r' as u16;
    }
    psz_new[ich_new] = 0;
    debug_assert_eq!(ich_new, psz_new.len() - 1);
}

/// ProcessNewLinesAndNulls — replace NULs with spaces, count newlines,
/// choose the dominant EOLN and normalize everything to CRLF.
/// Returns (new handle, text pointer, char count) or None on failure.
unsafe fn process_new_lines_and_nulls(
    h_local: HLOCAL,
    psz_text: *mut u16,
    cch_text: usize,
) -> Option<(HLOCAL, *mut u16, usize, Eoln)> {
    let mut adw_eoln_count = [0usize; 3]; // [CRLF, LF, CR]
    let mut b_prev_cr = false;

    // Replace '\0' with SPACE. Count newlines.
    for ich in 0..cch_text {
        let ch = *psz_text.add(ich);
        if ch == 0 {
            *psz_text.add(ich) = ' ' as u16;
        }
        if ch == '\n' as u16 {
            if b_prev_cr {
                adw_eoln_count[2] -= 1; // EOLN_CR
                adw_eoln_count[0] += 1; // EOLN_CRLF
            } else {
                adw_eoln_count[1] += 1; // EOLN_LF
            }
        } else if ch == '\r' as u16 {
            adw_eoln_count[2] += 1; // EOLN_CR
        }
        b_prev_cr = ch == '\r' as u16;
    }

    // Choose the newline code
    let i_eoln = if adw_eoln_count[2] > adw_eoln_count[0] {
        Eoln::Cr
    } else if adw_eoln_count[1] > adw_eoln_count[0] {
        Eoln::Lf
    } else {
        Eoln::CrLf
    };

    let c_non_crlfs = adw_eoln_count[2] + adw_eoln_count[1];
    if c_non_crlfs != 0 {
        // Allocate a buffer for EM_SETHANDLE
        let cch_new = cch_text + c_non_crlfs;
        let h_new = LocalAlloc(LMEM_MOVEABLE, (cch_new + 1) * 2).ok()?;
        let psz_new = LocalLock(h_new) as *mut u16;
        if psz_new.is_null() {
            let _ = LocalFree(Some(h_new));
            return None;
        }

        let old = std::slice::from_raw_parts(psz_text, cch_text);
        let new = std::slice::from_raw_parts_mut(psz_new, cch_new + 1);
        replace_new_lines(new, old);

        // Replace with new data (caller frees the old handle)
        Some((h_new, psz_new, cch_new, i_eoln))
    } else {
        Some((h_local, psz_text, cch_text, i_eoln))
    }
}

/// ReadText — read a whole file, detect encoding, normalize newlines.
/// Returns a LocalAlloc'ed handle suitable for EM_SETHANDLE.
pub unsafe fn read_text(
    h_file: HANDLE,
    penc_file: &mut Encoding,
    pi_eoln: &mut Eoln,
) -> Option<HLOCAL> {
    let dw_size = GetFileSize(h_file, None);
    if dw_size == INVALID_FILE_SIZE {
        return None;
    }

    // If file is empty
    if dw_size == 0 {
        let h_new = LocalAlloc(LMEM_MOVEABLE, 2).ok()?;
        let p = LocalLock(h_new) as *mut u16;
        *p = 0;
        let _ = LocalUnlock(h_new);
        *pi_eoln = Eoln::CrLf;
        *penc_file = ENCODING_DEFAULT;
        return Some(h_new);
    }

    let h_mapping = match CreateFileMappingW(h_file, None, PAGE_READONLY, 0, 0, None) {
        Ok(h) => h,
        Err(_) => return None,
    };
    let view = MapViewOfFile(h_mapping, FILE_MAP_READ, 0, 0, 0);
    if view.Value.is_null() {
        let _ = windows::Win32::Foundation::CloseHandle(h_mapping);
        return None;
    }
    let p_bytes = view.Value as *const u8;

    let bytes = std::slice::from_raw_parts(p_bytes, dw_size as usize);

    // Look for Byte Order Marks
    let mut dw_pos = 0usize;
    let enc_file;
    if dw_size >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        enc_file = Encoding::Utf16Le;
        dw_pos += 2;
    } else if dw_size >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        enc_file = Encoding::Utf16Be;
        dw_pos += 2;
    } else if dw_size >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        enc_file = Encoding::Utf8Bom;
        dw_pos += 3;
    } else {
        enc_file = analyze_encoding(bytes);
    }

    let mut result: Option<(HLOCAL, *mut u16, usize)> = None;

    match enc_file {
        Encoding::Utf16Be | Encoding::Utf16Le => {
            let cch_text = (dw_size as usize - dw_pos) / 2;
            if cch_text < (MAXLONG as usize) / 2 {
                if let Ok(h_new) = LocalAlloc(LMEM_MOVEABLE, (cch_text + 1) * 2) {
                    let p = LocalLock(h_new);
                    if !p.is_null() {
                        let psz_new = p as *mut u16;
                        let src = std::slice::from_raw_parts(
                            bytes.as_ptr().add(dw_pos) as *const u16,
                            cch_text,
                        );
                        let dst = std::slice::from_raw_parts_mut(psz_new, cch_text);
                        dst.copy_from_slice(src);
                        if enc_file == Encoding::Utf16Be {
                            // big endian; Swap bytes
                            for w in dst.iter_mut() {
                                *w = w.swap_bytes();
                            }
                        }
                        *psz_new.add(cch_text) = 0;
                        result = Some((h_new, psz_new, cch_text));
                    }
                }
            }
        }
        Encoding::Ansi | Encoding::Utf8 | Encoding::Utf8Bom => {
            let codepage = if enc_file == Encoding::Utf8 || enc_file == Encoding::Utf8Bom {
                CP_UTF8
            } else {
                CP_ACP
            };
            let cb_content = dw_size as usize - dw_pos;
            if cb_content < (MAXLONG as usize) / 2 {
                if let Ok(h_new) = LocalAlloc(LMEM_MOVEABLE, (cb_content + 1) * 2) {
                    let p = LocalLock(h_new);
                    if !p.is_null() {
                        let psz_new = p as *mut u16;
                        let mut cch_text = 0usize;
                        let mut ok = true;
                        if cb_content > 0 {
                            if let Some(wide) = multi_to_wide(codepage, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), &bytes[dw_pos..]) {
                                if wide.len() <= cb_content {
                                    std::ptr::copy_nonoverlapping(
                                        wide.as_ptr(),
                                        psz_new,
                                        wide.len(),
                                    );
                                    cch_text = wide.len();
                                } else {
                                    ok = false;
                                }
                            } else {
                                ok = false;
                            }
                        }
                        if ok {
                            *psz_new.add(cch_text) = 0;
                            result = Some((h_new, psz_new, cch_text));
                        }
                    }
                }
            }
        }
        _ => {}
    }

    let _ = UnmapViewOfFile(view);
    let _ = windows::Win32::Foundation::CloseHandle(h_mapping);

    let (h_local, psz_text, cch_text) = result?;

    match process_new_lines_and_nulls(h_local, psz_text, cch_text) {
        Some((h_new, _, _, i_eoln)) => {
            if h_new != h_local {
                let _ = LocalFree(Some(h_local));
            }
            *pi_eoln = i_eoln;
            *penc_file = enc_file;
            Some(h_new)
        }
        None => {
            let _ = LocalFree(Some(h_local));
            None
        }
    }
}

/// WriteEncodedText — encode and write a wide string to the file.
unsafe fn write_encoded_text(
    h_file: HANDLE,
    psz_text: &[u16],
    mut enc_file: Encoding,
) -> windows::core::Result<()> {
    use windows::Win32::Storage::FileSystem::WriteFile;
    use windows::Win32::System::Memory::{GetProcessHeap, HeapAlloc, HeapFree, HEAP_ZERO_MEMORY};

    let mut dw_pos = 0usize;

    while dw_pos < psz_text.len() {
        let p_bytes: Vec<u8>;
        match enc_file {
            Encoding::Utf16Le => {
                // LE is the native byte order on x86
                p_bytes = psz_text[dw_pos..]
                    .iter()
                    .flat_map(|w| w.to_le_bytes())
                    .collect();
                dw_pos = psz_text.len();
            }
            Encoding::Utf16Be => {
                p_bytes = psz_text[dw_pos..]
                    .iter()
                    .flat_map(|w| w.to_be_bytes())
                    .collect();
                dw_pos = psz_text.len();
            }
            Encoding::Ansi | Encoding::Utf8 | Encoding::Utf8Bom => {
                let codepage = if enc_file == Encoding::Utf8 || enc_file == Encoding::Utf8Bom {
                    CP_UTF8
                } else {
                    CP_ACP
                };
                match wide_to_multi(codepage, &psz_text[dw_pos..]) {
                    Some(bytes) => p_bytes = bytes,
                    None => {
                        return Err(windows::core::Error::from_hresult(windows::core::HRESULT::from_win32(
            GetLastError().0,
        )));
                    }
                }
                dw_pos = psz_text.len();
            }
            Encoding::Auto => {
                enc_file = ENCODING_DEFAULT;
                continue;
            }
        }

        if !p_bytes.is_empty() {
            let mut written = 0u32;
            WriteFile(h_file, Some(p_bytes.as_slice()), Some(&mut written), None)?;
        }
    }

    // keep HeapAlloc/HeapFree referenced for parity with the C version
    let _ = (GetProcessHeap, HeapAlloc, HeapFree, HEAP_ZERO_MEMORY);
    Ok(())
}

/// WriteText — write the whole text with the requested encoding and EOLN.
/// The in-edit text always uses CRLF; conversion happens here.
pub unsafe fn write_text(
    h_file: HANDLE,
    psz_text: &[u16],
    enc_file: Encoding,
    i_eoln: Eoln,
) -> windows::core::Result<()> {
    // Write the proper byte order marks if not ANSI or UTF-8 without BOM
    if enc_file != Encoding::Ansi && enc_file != Encoding::Utf8 {
        let bom: [u16; 1] = [0xFEFF];
        write_encoded_text(h_file, &bom, enc_file)?;
    }

    let mut dw_pos = 0usize;

    // pszText eoln are always \r\n
    loop {
        // Find the next eoln
        let mut dw_next = dw_pos;
        while dw_next < psz_text.len() {
            if psz_text[dw_next] == '\r' as u16
                && dw_next + 1 < psz_text.len()
                && psz_text[dw_next + 1] == '\n' as u16
            {
                break;
            }
            dw_next += 1;
        }

        if dw_next != psz_text.len() {
            match i_eoln {
                Eoln::Lf => {
                    // Write text (without eoln)
                    write_encoded_text(h_file, &psz_text[dw_pos..dw_next], enc_file)?;
                    // Write eoln
                    write_encoded_text(h_file, &['\n' as u16], enc_file)?;
                }
                Eoln::Cr => {
                    // Write text (including \r as eoln)
                    write_encoded_text(h_file, &psz_text[dw_pos..dw_next + 1], enc_file)?;
                }
                Eoln::CrLf => {
                    // Write text (including \r\n as eoln)
                    write_encoded_text(h_file, &psz_text[dw_pos..dw_next + 2], enc_file)?;
                }
            }
        } else {
            // Write text (without eoln, since this is the end of the file)
            write_encoded_text(h_file, &psz_text[dw_pos..dw_next], enc_file)?;
        }

        // Skip \r\n
        dw_pos = dw_next + 2;
        if dw_pos >= psz_text.len() {
            break;
        }
    }

    Ok(())
}




