//! Release builds are GUI-subsystem programs, so the terminal mode finds itself a console:
//! the Windows Terminal tab that launched it, a fresh Windows Terminal, or a classic console.

use std::io;

use crate::mode::{self, AppDirs};

pub enum Console {
    /// Standard input and output now belong to a console; draw the interface here.
    Ready,
    /// Another window took over; this process should exit.
    Relaunched,
}

#[cfg(windows)]
pub fn prepare(dirs: &AppDirs) -> io::Result<Console> {
    use windows_sys::Win32::System::Console::{AttachConsole, GetConsoleWindow, ATTACH_PARENT_PROCESS};

    // SAFETY: plain Win32 calls without pointer arguments.
    if unsafe { !GetConsoleWindow().is_null() } {
        return Ok(Console::Ready);
    }
    if mode::has_flag(mode::HOSTED_FLAG) {
        // SAFETY: as above; failure just leaves us without a console, handled below.
        if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0 {
            adopt_console_handles();
            return Ok(Console::Ready);
        }
    } else if let Some(terminal) = crate::terminal_host::find(&dirs.cache) {
        crate::terminal_host::launch(&terminal, &std::env::current_exe()?)?;
        return Ok(Console::Relaunched);
    }
    open_classic_console();
    Ok(Console::Ready)
}

#[cfg(not(windows))]
pub fn prepare(_dirs: &AppDirs) -> io::Result<Console> {
    Ok(Console::Ready)
}

/// Last resort without Windows Terminal: no Sixel, but everything else works.
#[cfg(windows)]
fn open_classic_console() {
    use windows_sys::Win32::System::Console::{AllocConsole, SetConsoleTitleW};

    let title: Vec<u16> = "SCARPIFY".encode_utf16().chain(Some(0)).collect();
    // SAFETY: the title is NUL-terminated and outlives the call.
    unsafe {
        AllocConsole();
        SetConsoleTitleW(title.as_ptr());
    }
    adopt_console_handles();
}

/// A GUI process starts without standard handles; point them at the console just joined.
#[cfg(windows)]
fn adopt_console_handles() {
    use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    use windows_sys::Win32::System::Console::{SetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};

    let open = |name: &str| {
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: the name is NUL-terminated; null security attributes and template are allowed.
        unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        }
    };
    let input = open("CONIN$");
    let output = open("CONOUT$");
    // SAFETY: the handles stay open for the life of the process, as standard handles must.
    unsafe {
        if input != INVALID_HANDLE_VALUE {
            SetStdHandle(STD_INPUT_HANDLE, input);
        }
        if output != INVALID_HANDLE_VALUE {
            SetStdHandle(STD_OUTPUT_HANDLE, output);
            SetStdHandle(STD_ERROR_HANDLE, output);
        }
    }
}
