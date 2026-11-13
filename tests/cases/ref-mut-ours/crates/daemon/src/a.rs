struct Log {
    head: HostSeq,
}

async fn fill(reader: Reader, buf: &mut Vec<u8>) -> Result<usize, Lost> {
    reader.read(buf).await.map_err(Lost::Read)
}
