use std::io::{Seek, Read, Result as IOResult, Error as IOError};
use std::ffi::{CString, NulError};
use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum StringReadError {
  IOError(IOError),
  StringConstructionError(FromUtf8Error)
}

pub trait StringRead {
  fn read_null_terminated_string(&mut self) -> Result<String, StringReadError>;
}

impl<T: Read> StringRead for T {
  fn read_null_terminated_string(&mut self) -> Result<String, StringReadError> {
    let mut buffer = Vec::<u8>::new();
    loop {
      let char = self.read_u8().map_err(|e| StringReadError::IOError(e))?;
      if char == 0 {
        break;
      }
      buffer.push(char);
    }
    String::from_utf8(buffer).map_err(|e| StringReadError::StringConstructionError(e))
  }
}

pub trait RawDataRead {
  fn read_data(&mut self, len: usize) -> IOResult<Box<[u8]>>;
}

impl<T: Read> RawDataRead for T {
  fn read_data(&mut self, len: usize) -> IOResult<Box<[u8]>> {
    let mut buffer = Vec::with_capacity(len);
    unsafe { buffer.set_len(len); }
    self.read_exact(&mut buffer)?;
    Ok(buffer.into_boxed_slice())
  }
}

pub trait PrimitiveRead {
  fn read_u8(&mut self) -> IOResult<u8>;
  fn read_u16(&mut self) -> IOResult<u16>;
  fn read_u32(&mut self) -> IOResult<u32>;
  fn read_u64(&mut self) -> IOResult<u64>;
  fn read_i8(&mut self) -> IOResult<i8>;
  fn read_i16(&mut self) -> IOResult<i16>;
  fn read_i32(&mut self) -> IOResult<i32>;
  fn read_i64(&mut self) -> IOResult<i64>;
  fn read_f32(&mut self) -> IOResult<f32>;
  fn read_f64(&mut self) -> IOResult<f64>;
}

impl<T: Read> PrimitiveRead for T {
  fn read_u8(&mut self) -> IOResult<u8> {
    let mut buffer = [0u8; 1];
    self.read_exact(&mut buffer)?;
    Ok(u8::from_le_bytes(buffer))
  }

  fn read_u16(&mut self) -> IOResult<u16> {
    let mut buffer = [0u8; 2];
    self.read_exact(&mut buffer)?;
    Ok(u16::from_le_bytes(buffer))
  }

  fn read_u32(&mut self) -> IOResult<u32> {
    let mut buffer = [0u8; 4];
    self.read_exact(&mut buffer)?;
    Ok(u32::from_le_bytes(buffer))
  }

  fn read_u64(&mut self) -> IOResult<u64> {
    let mut buffer = [0u8; 8];
    self.read_exact(&mut buffer)?;
    Ok(u64::from_le_bytes(buffer))
  }

  fn read_i8(&mut self) -> IOResult<i8> {
    let mut buffer = [0u8; 1];
    self.read_exact(&mut buffer)?;
    Ok(i8::from_le_bytes(buffer))
  }

  fn read_i16(&mut self) -> IOResult<i16> {
    let mut buffer = [0u8; 2];
    self.read_exact(&mut buffer)?;
    Ok(i16::from_le_bytes(buffer))
  }

  fn read_i32(&mut self) -> IOResult<i32> {
    let mut buffer = [0u8; 4];
    self.read_exact(&mut buffer)?;
    Ok(i32::from_le_bytes(buffer))
  }

  fn read_i64(&mut self) -> IOResult<i64> {
    let mut buffer = [0u8; 8];
    self.read_exact(&mut buffer)?;
    Ok(i64::from_le_bytes(buffer))
  }

  fn read_f32(&mut self) -> IOResult<f32> {
    let mut buffer = [0u8; 4];
    self.read_exact(&mut buffer)?;
    Ok(f32::from_le_bytes(buffer))
  }

  fn read_f64(&mut self) -> IOResult<f64> {
    let mut buffer = [0u8; 8];
    self.read_exact(&mut buffer)?;
    Ok(f64::from_le_bytes(buffer))
  }
}
