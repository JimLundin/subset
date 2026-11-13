enum Upgrade {
    Waiting(Pin<Box<dyn Future<Output = ()>>>),
}
