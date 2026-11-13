use tokio::process::Command;

use crate::a::{Attempt, RecvHalf};

async fn race(first: Attempt) -> Result<Connection, Lost> {
    let Attempt { endpoint, mut connecting } = first;
    let _ = endpoint;
    connecting.poll().await
}

async fn read(mut recv: RecvHalf) -> Frame {
    recv.read().await
}

fn command() -> Command {
    let mut cmd = Command::new("ls");
    cmd.kill_on_drop(true);
    cmd
}
