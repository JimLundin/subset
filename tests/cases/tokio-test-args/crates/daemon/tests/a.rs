#[tokio::test]
async fn replays() {
    assert_eq!(replay().await, Some(3));
}
