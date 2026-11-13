#[tokio::test(flavor = "multi_thread")]
async fn replays() {
    assert_eq!(replay().await, Some(3));
}
