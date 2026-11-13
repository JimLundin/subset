use tokio::process::Command;

#[cfg(not(unix))]
pub(crate) fn no_window(mut cmd: Command) -> Command {
    use std::os::windows::process::CommandExt;
    /// `CREATE_NO_WINDOW` from the Win32 process creation flags.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(unix)]
pub(crate) fn no_window(cmd: Command) -> Command {
    cmd
}
