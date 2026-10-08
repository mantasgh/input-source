use std::error;
use std::ffi::c_void;
use std::fmt;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use crate::cf_base::{CFAllocatorRef, CFIndex};
use crate::cf_retained::CFRetained;
use crate::mac_types::Boolean;

pub type CFURLRef = *const c_void;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFURLCreateFromFileSystemRepresentation(
        allocator: CFAllocatorRef,
        buffer: *const u8,
        bufLen: CFIndex,
        isDirectory: Boolean,
    ) -> CFURLRef;
}

#[derive(Debug)]
pub enum PathToCFURLError {
    Null,
    TooLong,
}

impl fmt::Display for PathToCFURLError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "CFURLCreateFromFileSystemRepresentation returned null"),
            Self::TooLong => write!(f, "path length exceeds CFIndex::MAX"),
        }
    }
}

impl error::Error for PathToCFURLError {}

pub struct CFURL(CFRetained);

impl CFURL {
    pub fn as_ptr(&self) -> CFURLRef {
        self.0.as_ptr()
    }

    pub fn from_path(path: &Path, is_directory: bool) -> Result<Self, PathToCFURLError> {
        let path = path.as_os_str().as_bytes();
        let length = CFIndex::try_from(path.len()).map_err(|_| PathToCFURLError::TooLong)?;
        let is_directory = if is_directory { 1 } else { 0 };

        let url = unsafe {
            CFURLCreateFromFileSystemRepresentation(
                std::ptr::null(),
                path.as_ptr(),
                length,
                is_directory,
            )
        };

        let retained = unsafe { CFRetained::from_raw(url) };
        retained.map(Self).ok_or(PathToCFURLError::Null)
    }
}
