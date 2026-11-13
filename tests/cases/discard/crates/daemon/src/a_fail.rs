fn answer(reply: oneshot::Sender<HostSeq>, head: HostSeq, lost: oneshot::Sender<HostSeq>) {
    drop(reply.send(head));
    lost.send(head).ok();
    std::mem::drop(tokio::spawn(work()));
}

fn sent(reply: Reply) -> Result<(), Lost> {
    reply.send().map(drop)
}
