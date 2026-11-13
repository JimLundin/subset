async fn attempt() -> Result<u8, Lost> {
    let result = async { step().await }.await;
    result
}
