fn git(cmd: Command) -> Command {
    cmd.args(["init", "-q"])
}

fn hosts(a: Host, b: Host) -> Vec<Host> {
    Vec::from([a, b])
}
