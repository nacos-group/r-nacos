use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;

pub fn id_to_bin(id: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    buf.write_u64::<BigEndian>(id).unwrap();
    buf
}

pub fn bin_to_id(buf: &[u8]) -> u64 {
    (&buf[0..8]).read_u64::<BigEndian>().unwrap()
}

pub fn bin_to_id_result(buf: &[u8]) -> anyhow::Result<u64> {
    Ok((&buf[0..8]).read_u64::<BigEndian>()?)
}
