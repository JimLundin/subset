async fn body(mut reader: Reader) -> Result<Vec<u8>, Lost> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await.map_err(Lost::Read)?;
    Ok(buf)
}
