#![crate_name = "var_io"]
//! A crate that adds 2 traits: VarRead and VarWrite. These traits are super-traits of Read and
//! Write, respectively, that add methods to read and write VarInts and VarStrings.
//!
//! *NOTE*: This should not be treated as compliant with Google's ProtoBuf! Rather, it was created
//! for the sole purpose of implementing the Minecraft (Java Edition) network protocol.

use std::io::{Read, Write};
use std::ops::*;

const VARINT_DONE_MASK: u8 = 0x80;
const VARINT_DATA_MASK: u8 = 0x7F;

/// A marker trait that is explicitly satisfied by the built-in integral types: 
///
/// * `u8`
/// * `u16`
/// * `u32`
/// * `u64`
/// * `i8`
/// * `i16`
/// * `i32`
/// * `i64`
/// * `isize`
/// * `usize`
///
///
/// WARN: This trait should not be manually implemented for any other type! It is meant for internal
/// use only and adds just the necessary bounds on what's needed to implement the [`VarRead`] and
/// [`VarWrite`] supertraits for [`Read`]ers and [`Write`]rs.
pub unsafe trait Integral: Sized + BitAnd + BitOr + Eq + Ord + Copy {}

unsafe impl Integral for u8 {}
unsafe impl Integral for u16 {}
unsafe impl Integral for u32 {}
unsafe impl Integral for u64 {}

unsafe impl Integral for i8 {}
unsafe impl Integral for i16 {}
unsafe impl Integral for i32 {}
unsafe impl Integral for i64 {}

unsafe impl Integral for usize {}
unsafe impl Integral for isize {}

impl<T: Read> VarRead for T {}
impl<T: Write> VarWrite for T {}

/// A supertrait of `Read` which adds ready-made methods for reading VarInts and VarStrings, as per
/// the Minecraft: Java edition server protocol specification.
pub trait VarRead: Read {
    /// Attempts to read a VarInt from the stream, returning an error if there were issues reading
    /// or parsing the data into an integer.
    fn read_var_int<I: Integral>(&mut self) -> Result<I, VarError> {
        let mut result: u64 = 0;
        let mut position: usize = 0;
        let mut bytes = self.bytes();
        let bit_size = std::mem::size_of::<I>() * 8;
        let max_bytes = (bit_size as f64 / 7.0).ceil() as usize;

        while let Some(res) = bytes.next() {
            let current_byte = res.map_err(|_| VarError::IOError)?;

            result |= (((current_byte & VARINT_DATA_MASK) as u64) << position) as u64;

            // If the MSB is 1, there is more to this integer!
            if current_byte & VARINT_DONE_MASK == 0 {
                let buf = result.to_ne_bytes();
                let view: &[I] =
                    unsafe { std::slice::from_raw_parts((&buf as *const u8) as *const I, 8) };
                return Ok(view[0]);
            }

            position += 7;

            if position >= max_bytes * 7 {
                return Err(VarError::InvalidVarInt);
            }
        }

        Err(VarError::EarlyEOF)
    }

    /// Attempts to read a VarString from the stream, returning an error if there were issues
    /// reading or converting the bytes into a UTF-8 String
    fn read_var_string(&mut self) -> Result<String, VarError> {
        let length: i32 = self.read_var_int()?;
        let mut s = vec![0u8; length as usize];
        self.read_exact(&mut s).map_err(|_| VarError::IOError)?;
        String::from_utf8(s).map_err(|_| VarError::InvalidVarString)
    }
}

/// A supertrait of `Write` which adds ready-made methods for writing VarInts and VarStrings, as per
/// the Minecraft: Java edition server protocol specification.
pub trait VarWrite: Write {
    /// Attempts to write a VarInt, returning an error if there were issues with writing
    fn write_var_int<I: Integral>(&mut self, value: I) -> Result<(), VarError> {
        let buf = [0u8; 8];
        let u: &mut [I] =
            unsafe { std::slice::from_raw_parts_mut((&buf as *const u8) as *mut I, 1) };
        u[0] = value;

        let mut value = u64::from_ne_bytes(buf);
        loop {
            if value & !(VARINT_DATA_MASK as u64) == 0 {
                self.write_all(&[value as u8])
                    .map_err(|_| VarError::IOError)?;
                return Ok(());
            }

            self.write_all(&[(value as u8 & VARINT_DATA_MASK) | VARINT_DONE_MASK])
                .map_err(|_| VarError::IOError)?;

            value >>= 7;
        }
    }

    /// Attempts to write a VarInt, returning an error if there were issues with writing
    fn write_var_string(&mut self, string: &str) -> Result<(), VarError> {
        self.write_var_int(string.len() as i32)
            .map_err(|_| VarError::IOError)?;

        self.write_all(string.as_bytes())
            .map_err(|_| VarError::IOError)?;

        Ok(())
    }

    /// All packets are prefixed with their length. This method simply takes a given response,
    /// writes its length as a VarInt, then writes the response itself. This method will call
    /// `.flush()` on the writer to ensure the full packet is sent, in the case of buffered
    /// writers.
    fn write_response(&mut self, response: &[u8]) -> Result<(), VarError> {
        self.write_var_int(response.len() as i32)?;
        self.write_all(response).map_err(|_| VarError::IOError)?;
        self.flush().map_err(|_| VarError::IOError)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VarError {
    EarlyEOF,
    IOError,
    InvalidVarInt,
    InvalidVarString,
}

impl std::fmt::Display for VarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
    }
}
impl std::error::Error for VarError {}

#[cfg(test)]
mod test {
    // TODO: Add tests to explicitly test all integer types!
    use super::*;
    #[test]
    fn test_read() {
        let v: Vec<u8> = vec![0xC8, 0x01];

        let mut r = v.as_slice();

        let x = r.read_var_int::<i32>();
        println!("{x:?}");
    }

    #[test]
    fn test_write() {
        let mut v: Vec<u8> = vec![];

        v.write_var_int::<i32>(-128).unwrap();
        for x in v {
            print!("0x{x:02x}, ");
        }
        println!();
    }
}
