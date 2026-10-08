use std::error::Error;
use std::ffi::c_void;
use std::fmt;
use std::ptr::null;

use crate::cf_base::{CFAllocatorRef, CFIndex};
use crate::cf_retained::CFRetained;
use crate::mac_types::Boolean;

pub type CFStringRef = *const c_void;
type CFStringEncoding = u32;

#[allow(non_upper_case_globals)]
const kCFStringEncodingUTF8: CFStringEncoding = 0x0800_0100;

#[repr(C)]
struct CFRange {
    location: CFIndex,
    length: CFIndex,
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithBytes(
        alloc: CFAllocatorRef,
        bytes: *const u8,
        numBytes: CFIndex,
        encoding: CFStringEncoding,
        isExternalRepresentation: Boolean,
    ) -> CFStringRef;
    fn CFStringGetLength(string: CFStringRef) -> CFIndex;
    fn CFStringGetMaximumSizeForEncoding(length: CFIndex, encoding: CFStringEncoding) -> CFIndex;
    fn CFStringGetBytes(
        string: CFStringRef,
        range: CFRange,
        encoding: CFStringEncoding,
        lossByte: u8,
        isExternalRepresentation: Boolean,
        buffer: *mut u8,
        maxBufLen: CFIndex,
        usedBufLen: *mut CFIndex,
    ) -> CFIndex;
}

#[derive(Debug)]
pub enum StrToCFStringError {
    TooLong,
    Null,
}

impl fmt::Display for StrToCFStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => write!(f, "string byte length exceeds CFIndex::MAX"),
            Self::Null => write!(f, "CFStringCreateWithBytes returned null"),
        }
    }
}

impl Error for StrToCFStringError {}

pub struct CFString(CFRetained);

impl CFString {
    pub fn as_ptr(&self) -> CFStringRef {
        self.0.as_ptr()
    }

    pub unsafe fn from_borrowed(ptr: CFStringRef) -> Option<Self> {
        let retained = unsafe { CFRetained::from_borrowed(ptr) };
        retained.map(Self)
    }
}

impl From<CFString> for CFRetained {
    fn from(string: CFString) -> Self {
        string.0
    }
}

impl TryFrom<&str> for CFString {
    type Error = StrToCFStringError;

    fn try_from(string: &str) -> Result<Self, Self::Error> {
        let length = CFIndex::try_from(string.len()).map_err(|_| Self::Error::TooLong)?;

        let ptr = unsafe {
            CFStringCreateWithBytes(null(), string.as_ptr(), length, kCFStringEncodingUTF8, 0)
        };

        let retained = unsafe { CFRetained::from_raw(ptr) };
        retained.map(Self).ok_or(Self::Error::Null)
    }
}

#[derive(Debug)]
pub enum CFStringToStringError {
    InvalidSize,
    ConversionFailed,
    InvalidUtf8(std::string::FromUtf8Error),
}

impl fmt::Display for CFStringToStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize => {
                write!(
                    f,
                    "CFStringGetMaximumSizeForEncoding returned a negative size"
                )
            }
            Self::ConversionFailed => {
                write!(
                    f,
                    "CFStringGetBytes failed to convert the entire string to UTF-8"
                )
            }
            Self::InvalidUtf8(_) => {
                write!(f, "CFStringGetBytes produced invalid UTF-8")
            }
        }
    }
}

impl Error for CFStringToStringError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUtf8(err) => Some(err),
            Self::InvalidSize | Self::ConversionFailed => None,
        }
    }
}

impl TryInto<String> for CFString {
    type Error = CFStringToStringError;

    fn try_into(self) -> Result<String, CFStringToStringError> {
        let length = unsafe { CFStringGetLength(self.as_ptr()) };

        let maximum_size =
            unsafe { CFStringGetMaximumSizeForEncoding(length, kCFStringEncodingUTF8) };

        if maximum_size < 0 {
            return Err(CFStringToStringError::InvalidSize);
        }

        let mut buffer = vec![0_u8; maximum_size as usize];
        let mut used_bytes: CFIndex = 0;
        let range = CFRange {
            location: 0,
            length,
        };

        let converted = unsafe {
            CFStringGetBytes(
                self.as_ptr(),
                range,
                kCFStringEncodingUTF8,
                0,
                0,
                buffer.as_mut_ptr(),
                maximum_size,
                &mut used_bytes,
            )
        };

        if converted != length {
            return Err(Self::Error::ConversionFailed);
        }

        buffer.truncate(used_bytes as usize);
        String::from_utf8(buffer).map_err(Self::Error::InvalidUtf8)
    }
}
