use std::io::{Read, Write};

use anyhow::bail;

const CONTINUE_BIT_MASK: i64 = 0x80;
const SEGMENT_BITS_MASK: i64 = 0x7F;

pub fn read_var_int_optional<T: Read>(
    stream: &mut T,
    max_bytes: u8,
) -> anyhow::Result<Option<i32>> {
    let mut result: i32 = 0;
    let mut position: i32 = 0;
    let mut current_byte = [0u8; 1];
    loop {
        let bytes_read = stream.read(&mut current_byte)?;
        if bytes_read == 0 {
            if position == 0 { return Ok(None); } // Clean EOF
            else { bail!("Connection closed mid-VarInt"); }
        }

        result |= (current_byte[0] as i32 & SEGMENT_BITS_MASK as i32) << position;

        if current_byte[0] as i32 & CONTINUE_BIT_MASK as i32 == 0 {
            return Ok(Some(result));
        }

        position += 7;

        if position >= max_bytes as i32 * 7 {
            bail!("VarInt too big!");
        }
    }
}

pub fn read_var_int<T: Read>(stream: &mut T, max_bytes: u8) -> anyhow::Result<i32> {
    let mut result: i32 = 0;
    let mut position: i32 = 0;
    let mut current_byte = [0u8; 1];
    loop {
        stream.read_exact(&mut current_byte)?;

        result |= (current_byte[0] as i32 & SEGMENT_BITS_MASK as i32) << position;

        if current_byte[0] as i32 & CONTINUE_BIT_MASK as i32 == 0 {
            return Ok(result);
        }

        position += 7;

        if position >= max_bytes as i32 * 7 {
            bail!("VarInt too big!");
        }
    }
}

pub fn read_var_string<T: Read>(stream: &mut T) -> anyhow::Result<String> {
    let length = read_var_int(stream, 5)?;
    let mut s = vec![0u8; length as usize];
    stream.read_exact(&mut s)?;
    Ok(String::from_utf8(s)?)
}

pub fn write_var_int<T: Write>(stream: &mut T, mut value: i32) -> anyhow::Result<()> {
    loop {
        if value & !SEGMENT_BITS_MASK as i32 == 0 {
            stream.write_all(&[value as u8])?;
            return Ok(());
        }

        stream.write_all(&[(value & SEGMENT_BITS_MASK as i32 | CONTINUE_BIT_MASK as i32) as u8])?;

        value = ((value as u32) >> 7) as i32;
    }
}

pub fn write_var_string<T: Write>(stream: &mut T, string: &str) -> anyhow::Result<()> {
    write_var_int(stream, string.len() as i32)?;
    stream.write_all(string.as_bytes())?;
    Ok(())
}

pub fn write_response<T: Write>(stream: &mut T, response: &[u8]) -> anyhow::Result<()> {
    write_var_int(stream, response.len() as i32)?;
    stream.write_all(response)?;
    stream.flush()?;
    Ok(())
}
