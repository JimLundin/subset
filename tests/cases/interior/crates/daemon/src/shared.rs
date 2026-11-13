fn shared(count: Rc<RefCell<u64>>, flag: Arc<AtomicBool>) -> Mutex<u64> {
    Mutex::new(RefCell::new(0).into_inner())
}
