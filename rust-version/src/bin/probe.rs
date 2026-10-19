#![windows_subsystem = "windows"]
use windows::{core::*, Win32::UI::WindowsAndMessaging::*};

fn main() -> Result<()> {
    unsafe {
        MessageBoxW(None, w!("Toolchain Rust -> Windows OK !"), w!("probe"), MB_OK);
    }
    Ok(())
}
