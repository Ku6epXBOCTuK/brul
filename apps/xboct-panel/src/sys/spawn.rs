use std::collections::BTreeMap;
use std::mem::{size_of, zeroed};
use std::path::Path;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{
    CREATE_NEW_PROCESS_GROUP, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DETACHED_PROCESS,
    PROCESS_INFORMATION, ResumeThread, STARTF_USESHOWWINDOW, STARTUPINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

use super::{Error, Handle, command, text::wide};

const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
const CREATE_SUSPENDED: u32 = 0x0000_0004;

pub(crate) struct Spawned {
    pid: u32,
    process: Handle,
    thread: Thread,
}

impl Spawned {
    pub(crate) fn pid(&self) -> u32 {
        self.pid
    }

    pub(crate) fn process(&self) -> &Handle {
        &self.process
    }

    pub(crate) fn resume(self) -> Handle {
        let Self {
            process, thread, ..
        } = self;
        unsafe {
            ResumeThread(thread.0);
        }
        process
    }
}

struct Thread(*mut core::ffi::c_void);

impl Drop for Thread {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn run(
    exe: &str,
    line: &str,
    workdir: Option<&Path>,
    environment: &BTreeMap<String, String>,
    flags: u32,
    hidden: bool,
) -> Result<Spawned, Error> {
    let wide_exe = wide(exe);
    let mut wide_line = wide(line);
    let wide_workdir = workdir.map(|dir| wide(&dir.to_string_lossy()));
    let block = command::environment(environment);
    let mut startup: STARTUPINFOW = unsafe { zeroed() };
    startup.cb = size_of::<STARTUPINFOW>() as u32;
    if hidden {
        startup.dwFlags = STARTF_USESHOWWINDOW;
        startup.wShowWindow = SW_HIDE as u16;
    }
    let mut process: PROCESS_INFORMATION = unsafe { zeroed() };
    let done = unsafe {
        CreateProcessW(
            wide_exe.as_ptr(),
            wide_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            flags,
            block
                .as_ref()
                .map_or(std::ptr::null(), |block| block.as_ptr().cast()),
            wide_workdir
                .as_ref()
                .map_or(std::ptr::null(), |dir| dir.as_ptr()),
            &startup,
            &mut process,
        )
    };
    if done == 0 {
        return Err(Error::last());
    }
    Ok(Spawned {
        pid: process.dwProcessId,
        process: Handle::from_raw(process.hProcess),
        thread: Thread(process.hThread),
    })
}

pub(crate) fn spawn_detached(
    exe: &str,
    args: &[String],
    workdir: Option<&Path>,
    environment: &BTreeMap<String, String>,
) -> Result<(u32, Handle), Error> {
    let line = command::line(exe, args);
    let spawned = run(
        exe,
        &line,
        workdir,
        environment,
        DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_UNICODE_ENVIRONMENT,
        false,
    )?;
    let pid = spawned.pid();
    let process = spawned.resume();
    Ok((pid, process))
}

pub(crate) fn spawn_console(
    program: &str,
    args: &[String],
    workdir: Option<&Path>,
    environment: &BTreeMap<String, String>,
) -> Result<Spawned, Error> {
    let conhost = std::env::var("SystemRoot").unwrap_or_else(|_| String::from(r"C:\Windows"))
        + r"\System32\conhost.exe";
    let line = format!(
        "{} {}",
        command::quote(&conhost),
        command::line(program, args)
    );
    run(
        &conhost,
        &line,
        workdir,
        environment,
        CREATE_NEW_CONSOLE | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
        true,
    )
}
