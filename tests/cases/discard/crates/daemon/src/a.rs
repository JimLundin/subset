fn answer(reply: oneshot::Sender<HostSeq>, head: HostSeq, guard: Lease) {
    let _ = reply.send(head);
    drop(guard);
}
