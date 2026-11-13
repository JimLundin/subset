use bytes::Buf;

fn kind(message: &[u8]) -> Option<u16> {
    let mut header = message;
    header.try_get_u16().ok()
}

fn len(header: &[u8]) -> u16 {
    bytes::Buf::get_u16(&mut &header[2..])
}
