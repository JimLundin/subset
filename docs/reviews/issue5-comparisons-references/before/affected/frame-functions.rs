use std::cmp::Ordering;

pub(crate) const MAX_FRAME_LEN: u32 = 16_777_216;

pub fn encode<T: Serialize>(msg: &T) -> Result<Vec<u8>, FrameError> {
    let body = postcard::to_allocvec(msg)?;
    match u32::try_from(body.len()).map(|len| (len, len.cmp(&MAX_FRAME_LEN))) {
        Ok((len, Ordering::Less | Ordering::Equal)) => Ok(len
            .to_be_bytes()
            .into_iter()
            .chain(body)
            .collect::<Vec<u8>>()),
        Ok((_, Ordering::Greater)) | Err(_) => Err(FrameError::TooLarge(
            u64::try_from(body.len()).unwrap_or(u64::MAX),
        )),
    }
}

pub fn body_len(header: [u8; HEADER_LEN]) -> Result<usize, FrameError> {
    let len = u32::from_be_bytes(header);
    match len.cmp(&MAX_FRAME_LEN) {
        Ordering::Less | Ordering::Equal => {
            usize::try_from(len).map_err(|_| FrameError::TooLarge(u64::from(len)))
        }
        Ordering::Greater => Err(FrameError::TooLarge(u64::from(len))),
    }
}
