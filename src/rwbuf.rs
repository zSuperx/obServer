// Source - https://stackoverflow.com/a
// Posted by Chayim Friedman
// Retrieved 2026-01-01, License - CC BY-SA 4.0

use std::io::{BufRead, BufReader, BufWriter, IoSlice, IoSliceMut, Read, Result, Write};

pub struct MyBufReader<T: ?Sized> {
    inner: BufReader<T>,
}

impl<T: Read> MyBufReader<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: BufReader::new(inner),
        }
    }
}

impl<T: Read + ?Sized> Read for MyBufReader<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        self.inner.read_vectored(bufs)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.inner.read_exact(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.inner.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        self.inner.read_to_string(buf)
    }
}

impl<T: Read + ?Sized> BufRead for MyBufReader<T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt)
    }
}

impl<T: Write + ?Sized> Write for MyBufReader<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.get_mut().write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.inner.get_mut().write_all(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        self.inner.get_mut().write_vectored(bufs)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.get_mut().flush()
    }
}

pub struct MyBufWriter<T: ?Sized + Write> {
    inner: BufWriter<T>,
}

impl<T: Write> MyBufWriter<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: BufWriter::new(inner),
        }
    }
}

impl<T: Write + ?Sized> Write for MyBufWriter<T> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.inner.write_all(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        self.inner.write_vectored(bufs)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

impl<T: Write + Read + ?Sized> Read for MyBufWriter<T> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.get_mut().read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        self.inner.get_mut().read_vectored(bufs)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        self.inner.get_mut().read_exact(buf)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        self.inner.get_mut().read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        self.inner.get_mut().read_to_string(buf)
    }
}

impl<T: Write + BufRead + ?Sized> BufRead for MyBufWriter<T> {
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.get_mut().fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.get_mut().consume(amt)
    }
}
