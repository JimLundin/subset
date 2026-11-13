fn cancel(open: Option<Approval>) {
    if let Some(approval) = open {
        approval.cancel();
    }
}
