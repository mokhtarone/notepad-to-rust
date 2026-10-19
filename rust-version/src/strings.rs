//! English string table — port of the STRINGTABLE in en-US.rc.
//! `sw()` returns a NUL-terminated UTF-16 buffer (LoadString equivalent).

#![allow(dead_code)]

use crate::consts::*;

pub fn s(id: u32) -> &'static str {
    match id {
        STRING_PAGESETUP_HEADERVALUE => "&f",
        STRING_PAGESETUP_FOOTERVALUE => "Page &p",
        STRING_NOTEPAD => "Notepad",
        STRING_ERROR => "ERROR",
        STRING_WARNING => "WARNING",
        STRING_INFO => "Information",
        STRING_UNTITLED => "Untitled",
        STRING_ALL_FILES => "All files (*.*)",
        STRING_TEXT_FILES_TXT => "Text files (*.txt)",
        STRING_TOOLARGE => {
            "File '%s' is too large for notepad.\nPlease use a different editor."
        }
        STRING_NOTEXT => "You didn't enter any text.\nPlease type something and try again",
        STRING_DOESNOTEXIST => "File '%s'\ndoes not exist\n\nDo you want to create a new file?",
        STRING_NOTSAVED => "File '%s'\nhas been modified\n\nWould you like to save the changes?",
        STRING_NOTFOUND => "'%s' can not be found.",
        STRING_OUT_OF_MEMORY => {
            "Not enough memory to complete this task.\nClose one or more applications to increase the amount of\nfree memory."
        }
        STRING_CANNOTFIND => "Cannot find '%s'",
        STRING_ANSI => "ANSI",
        STRING_UNICODE => "Unicode",
        STRING_UNICODE_BE => "Unicode (big endian)",
        STRING_UTF8 => "UTF-8",
        STRING_UTF8_BOM => "UTF-8 with BOM",
        STRING_CRLF => "Windows (CR + LF)",
        STRING_LF => "Unix (LF)",
        STRING_CR => "Mac (CR)",
        STRING_LINE_COLUMN => "Line %d, column %d",
        STRING_PRINTERROR => {
            "Cannot print the file '%s'.\n\nMake sure that the printer is turned on and is configured properly."
        }
        STRING_DEFAULTFONT => "Lucida Console",
        STRING_LINE_NUMBER_OUT_OF_RANGE => "The specified line number is out of range.",
        STRING_NOWPRINTING => "Now printing page...",
        STRING_PRINTCANCELING => "The print job is being canceled...",
        STRING_PRINTCOMPLETE => "Printing is successfully done.",
        STRING_PRINTCANCELED => "Printing has been canceled.",
        STRING_PRINTFAILED => "Printing failed.",
        STRING_TEXT_DOCUMENT => "Text Document",
        STRING_NOTEPAD_AUTHORS => {
            "Copyright 1997,98 Marcel Baur, 2000 Mike McCormack, 2002 Sylvain Petreolle, 2002 Andriy Palamarchuk\r\n"
        }
        _ => "",
    }
}

/// LoadString equivalent: NUL-terminated UTF-16.
pub fn sw(id: u32) -> Vec<u16> {
    s(id).encode_utf16().chain(std::iter::once(0)).collect()
}

/// One argument of `format_sw`.
pub enum Arg<'a> {
    S(&'a [u16]), // NUL-terminated UTF-16 string
    I(i32),
    U(u32),
}

/// StringCchPrintf(szBuf, szResource, ...) equivalent: replaces each
/// `%s`, `%d` or `%u` in the format string with the next argument.
pub fn format_sw(id: u32, args: &[Arg]) -> Vec<u16> {
    format_str(s(id), args)
}

pub fn format_str(fmt: &str, args: &[Arg]) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::new();
    let mut argi = 0;
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c as u16);
            continue;
        }
        // skip flags/width/precision we don't support
        let mut spec = String::new();
        while let Some(&nc) = chars.peek() {
            if nc.is_ascii_digit() || nc == '-' || nc == '.' || nc == '+' {
                spec.push(nc);
                chars.next();
            } else {
                break;
            }
        }
        match chars.next() {
            Some('s') => {
                if argi < args.len() {
                    if let Arg::S(v) = &args[argi] {
                        let mut j = 0;
                        while j < v.len() && v[j] != 0 {
                            out.push(v[j]);
                            j += 1;
                        }
                    }
                    argi += 1;
                }
            }
            Some('d') | Some('i') => {
                let v = if argi < args.len() {
                    if let Arg::I(v) = args[argi] {
                        v
                    } else {
                        0
                    }
                } else {
                    0
                };
                argi += 1;
                for dc in format!("{}{}", spec, v).chars() {
                    out.push(dc as u16);
                }
            }
            Some('u') => {
                let v = if argi < args.len() {
                    if let Arg::U(v) = args[argi] {
                        v
                    } else {
                        0
                    }
                } else {
                    0
                };
                argi += 1;
                for dc in format!("{}{}", spec, v).chars() {
                    out.push(dc as u16);
                }
            }
            Some('%') => out.push('%' as u16),
            Some(other) => {
                out.push('%' as u16);
                out.push(other as u16);
            }
            None => out.push('%' as u16),
        }
    }
    out.push(0);
    out
}

/// Wide string helper: &str -> NUL-terminated UTF-16.
pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// NUL-terminated UTF-16 buffer -> String (for reading back Globals buffers).
pub fn to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// Maximum size of the file-type filter buffer (Globals.szFilter).
pub const MAX_FILTER: usize = 512;

