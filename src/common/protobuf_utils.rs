use std::io::SeekFrom;

use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub fn write_varint64(mut v: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5);
    while v > 0x7F {
        buf.push(((v as u8) & 0x7F) | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
    buf
}
pub fn read_varint64(bytes: &[u8]) -> anyhow::Result<u64> {
    read_varint64_offset(bytes, 0)
}

pub fn read_varint64_offset(bytes: &[u8], offset: usize) -> anyhow::Result<u64> {
    // part0
    let mut i = offset;
    let mut b = bytes[i];
    if b & 0x80 == 0 {
        return Ok(b as u64);
    }
    let mut r0 = (b & 0x7f) as u32;

    i += 1;
    b = bytes[i];
    r0 |= ((b & 0x7f) as u32) << 7;
    if b & 0x80 == 0 {
        return Ok(r0 as u64);
    }

    i += 1;
    b = bytes[i];
    r0 |= ((b & 0x7f) as u32) << 14;
    if b & 0x80 == 0 {
        return Ok(r0 as u64);
    }

    i += 1;
    b = bytes[i];
    r0 |= ((b & 0x7f) as u32) << 21;
    if b & 0x80 == 0 {
        return Ok(r0 as u64);
    }

    // part1
    i += 1;
    b = bytes[i];
    let mut r1 = (b & 0x7f) as u32;
    if b & 0x80 == 0 {
        return Ok(r0 as u64 | ((r1 as u64) << 28));
    }

    i += 1;
    b = bytes[i];
    r1 |= ((b & 0x7f) as u32) << 7;
    if b & 0x80 == 0 {
        return Ok(r0 as u64 | ((r1 as u64) << 28));
    }

    i += 1;
    b = bytes[i];
    r1 |= ((b & 0x7f) as u32) << 14;
    if b & 0x80 == 0 {
        return Ok(r0 as u64 | ((r1 as u64) << 28));
    }

    i += 1;
    b = bytes[i];
    r1 |= ((b & 0x7f) as u32) << 21;
    if b & 0x80 == 0 {
        return Ok(r0 as u64 | ((r1 as u64) << 28));
    }

    // part2
    i += 1;
    b = bytes[i];
    let mut r2 = (b & 0x7f) as u32;
    if b & 0x80 == 0 {
        return Ok(r0 as u64 | ((r1 as u64) << 28) | ((r2 as u64) << 56));
    }

    // WARNING ABOUT TRUNCATION:
    //
    // For the number to be within valid 64 bit range, some conditions about
    // this last byte must be met:
    // 1. This must be the last byte (MSB not set)
    // 2. No 64-bit overflow (middle 6 bits are beyond 64 bits for the
    //    entire varint, so they cannot be set either)
    //
    // However, for the sake of consistency with Google's own protobuf
    // implementation, and also to allow for any efficient use of those
    // extra bits by users if they wish (this crate is meant for speed
    // optimization anyway) we shall not check for this here.
    //
    // Therefore, THIS FUNCTION SIMPLY IGNORES THE EXTRA BITS, WHICH IS
    // ESSENTIALLY A SILENT TRUNCATION!
    i += 1;
    b = bytes[i];
    r2 |= (b as u32) << 7;
    if b & 0x80 == 0 {
        return Ok((r0 as u64 | ((r1 as u64) << 28)) | ((r2 as u64) << 56));
    }

    // cannot read more than 10 bytes
    Err(anyhow::anyhow!("can't read more than 10 bytes"))
}

pub fn inner_sizeof_varint(v: u64) -> usize {
    match v {
        0x0..=0x7F => 1,
        0x80..=0x3FFF => 2,
        0x4000..=0x1FFFFF => 3,
        0x200000..=0xFFFFFFF => 4,
        0x10000000..=0x7FFFFFFFF => 5,
        0x0800000000..=0x3FFFFFFFFFF => 6,
        0x040000000000..=0x1FFFFFFFFFFFF => 7,
        0x02000000000000..=0xFFFFFFFFFFFFFF => 8,
        0x0100000000000000..=0x7FFFFFFFFFFFFFFF => 9,
        _ => 10,
    }
}

fn move_data_to_start(message_buf: &mut [u8], start: usize) {
    if start == 0 {
        return;
    }
    for i in start..message_buf.len() {
        message_buf[i - start] = message_buf[i];
    }
}

fn copy_data(form: &[u8], to: &mut [u8], start: usize) {
    let len = form.len();
    to[start..(len + start)].copy_from_slice(&form[..len]);
    /*
    for i in 0..len {
        to[start + i] = form[i]
    }
     */
}

#[derive(Debug)]
pub struct MessageBufReader {
    pub(crate) buf: Vec<u8>,
    start: usize,
    end: usize,
    //enough_next: bool,
    next_len: usize,
}

impl Default for MessageBufReader {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBufReader {
    pub fn new() -> Self {
        Self {
            buf: vec![0u8; 1024],
            start: 0,
            end: 0,
            //enough_next: false,
            next_len: 0,
        }
    }

    pub fn new_with_data(buf: Vec<u8>, start: usize) -> Self {
        let end = buf.len();
        let next_len = end - start;
        Self {
            buf,
            start,
            end,
            next_len,
        }
    }

    pub fn is_empty(&self) -> bool {
        if self.start >= self.buf.len() {
            true
        } else {
            self.buf[self.start] == 0
        }
    }

    pub fn append_next_buf(&mut self, next_buf: &[u8]) {
        move_data_to_start(&mut self.buf, self.start);
        self.end -= self.start;
        self.start = 0;
        //扩容
        while self.buf.len() - self.end < next_buf.len() {
            self.capacity_expansion();
        }
        copy_data(next_buf, &mut self.buf, self.end);
        self.end += next_buf.len();
    }

    fn capacity_expansion(&mut self) {
        let mut v = vec![0u8; self.buf.len()];
        self.buf.append(&mut v);
    }

    pub fn next_message_vec(&mut self) -> Option<&[u8]> {
        let mut i = self.start;
        let mut can_read_len = false;
        if self.is_empty() {
            return None;
        }
        while i < self.end {
            if self.buf[i] & 0x80 == 0 {
                can_read_len = true;
                i += 1;
                break;
            }
            i += 1;
        }
        if can_read_len {
            let r = &self.buf[self.start..self.end];
            if let Ok(s) = read_varint64(r) {
                self.next_len = i - self.start + (s as usize);
            }
            if self.end - self.start >= self.next_len {
                let v = &r[0..self.next_len];
                self.start += self.next_len;
                self.next_len = 0;
                return Some(v);
            }
        }
        None
    }
}

#[derive(Debug, Default)]
pub struct MessagePosition {
    pub position: u64,
    pub len: u64,
}

impl MessagePosition {
    pub fn get_end_position(&self) -> u64 {
        self.position + self.len
    }
}

pub struct FileMessageReader {
    file: tokio::fs::File,
    start: u64,
}

impl FileMessageReader {
    pub fn new(file: tokio::fs::File, start: u64) -> Self {
        Self { file, start }
    }

    pub async fn seek_start(&mut self, start: u64) -> anyhow::Result<()> {
        self.file.seek(SeekFrom::Start(start)).await?;
        self.start = start;
        Ok(())
    }

    pub async fn read_next(&mut self) -> anyhow::Result<Vec<u8>> {
        let len = self.read_len().await?;
        let mut data_buf = vec![0u8; len as usize];
        let data_len = self.file.read(&mut data_buf).await?;
        if data_len < data_buf.len() {
            return Err(anyhow::anyhow!("read data not enough"));
        }
        self.start += data_len as u64;
        Ok(data_buf)
    }

    pub async fn read_by_position(&mut self, position: (u64, usize)) -> anyhow::Result<Vec<u8>> {
        let start = self.start;
        let len = position.1 as u64;
        let mut data_buf = vec![0u8; len as usize];
        self.file.seek(SeekFrom::Start(position.0)).await?;
        let data_len = self.file.read(&mut data_buf).await?;
        if data_len < data_buf.len() {
            return Err(anyhow::anyhow!("read data not enough"));
        }
        self.file.seek(SeekFrom::Start(start)).await?;
        Ok(data_buf)
    }

    pub async fn read_to_end(&mut self) -> anyhow::Result<(u64, MessagePosition)> {
        let mut count = 0;
        let mut last_position = MessagePosition {
            position: self.start,
            len: 0,
        };
        while let Ok(msg_position) = self.read_next_position().await {
            count += 1;
            last_position = msg_position;
        }
        Ok((count, last_position))
    }

    pub async fn read_next_position(&mut self) -> anyhow::Result<MessagePosition> {
        let start = self.start;
        let len = self.read_len().await?;
        self.start += len;
        self.file.seek(SeekFrom::Start(self.start)).await?;
        Ok(MessagePosition {
            position: start,
            len,
        })
    }

    pub async fn read_index_position(&mut self, index: usize) -> anyhow::Result<MessagePosition> {
        for _ in 0..index {
            self.read_next_position().await?;
        }
        self.read_next_position().await
    }

    async fn read_len(&mut self) -> anyhow::Result<u64> {
        let mut len_buf = [0u8; 10];
        let read_len = self.file.read(&mut len_buf).await?;
        if read_len == 0 {
            return Err(anyhow::anyhow!("read end,position:{}", self.start));
        }
        let len = read_varint64(&len_buf)?;
        if len == 0 {
            return Err(anyhow::anyhow!("read end,position:{}", self.start));
        }
        self.file.seek(SeekFrom::Start(self.start)).await?;
        Ok(len + inner_sizeof_varint(len) as u64)
    }
}
