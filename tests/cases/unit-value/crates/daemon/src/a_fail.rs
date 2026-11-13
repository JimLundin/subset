enum Done {
    Now(Signal),
}

fn finish() -> Done {
    Done::Now(())
}
