// Tests supplement exact source functions in reproduce.py. They do not
// represent new product code or claim a full installer compilation.
fn failure() -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "same cause"))
}

fn describe(result: Result<(), InstallError>) -> (String, std::io::ErrorKind, String) {
    let InstallError::Io { path, error } = result.unwrap_err();
    (path, error.kind(), error.to_string())
}

#[test]
fn repeated_local_helpers_and_fresh_factories_produce_the_same_owned_error() {
    let owner = PathBuf::from("parent/child");
    let path = owner.as_path();
    let path_error = |error| io(path, error);
    let before_first = describe(failure().map_err(path_error));
    let before_second = describe(failure().map_err(path_error));
    let after_first = describe(failure().map_err(io_error(path)));
    let after_second = describe(failure().map_err(io_error(path)));
    assert_eq!((before_first, before_second), (after_first, after_second));
}

#[test]
fn pathbuf_and_temporary_path_borrows_finish_at_the_map_err_expression() {
    let mut lock_path = PathBuf::from("directory/lock");
    let result = failure().map_err(io_error(&lock_path));
    lock_path.push("later");
    assert_eq!(describe(result).0, "directory/lock");
    let result = failure().map_err(io_error(&PathBuf::from("temporary")));
    assert_eq!(describe(result).0, "temporary");
}

#[test]
fn success_does_not_construct_an_error_and_hold_keeps_the_direct_constructor() {
    let path = Path::new("successful");
    assert!(Ok::<_, std::io::Error>(()).map_err(io_error(path)).is_ok());
    assert_eq!(describe(Err(io(path, std::io::Error::other("held")))).0, "successful");
}

#[test]
fn actual_synchronous_remove_callers_retain_the_keep_and_lock_paths() {
    let root = std::env::current_dir().unwrap().join(format!("remove-test-{}", std::process::id()));
    std::fs::create_dir_all(root.join("keep")).unwrap();
    std::fs::create_dir_all(root.join("old")).unwrap();
    std::fs::write(root.join("lock"), "held").unwrap();
    std::fs::write(root.join("download"), "partial").unwrap();
    remove_others(&root, "keep").unwrap();
    assert!(root.join("keep").is_dir());
    assert!(root.join("lock").is_file());
    assert!(!root.join("old").exists());
    assert!(!root.join("download").exists());
    remove(&root).unwrap();
}

#[test]
fn actual_read_dir_failure_keeps_its_path_and_io_cause() {
    let path = std::env::current_dir().unwrap().join("missing-issue5-factory-directory");
    let before = std::fs::read_dir(&path).map_err(|error| io(&path, error)).unwrap_err();
    let after = remove_others(&path, "keep").unwrap_err();
    assert_eq!(describe(Err(before)), describe(Err(after)));
}

#[test]
fn awaited_result_mapping_accepts_the_same_factory_after_the_await() {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    let path = Path::new("awaited");
    let mut future = std::pin::pin!(async {
        std::future::ready(failure()).await.map_err(io_error(path))
    });
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(result) => assert_eq!(describe(result).0, "awaited"),
        Poll::Pending => panic!("a ready Result must finish immediately"),
    }
}
