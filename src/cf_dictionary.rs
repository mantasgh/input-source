use std::error;
use std::ffi::c_void;
use std::fmt;

use crate::cf_base::{CFAllocatorRef, CFIndex, CFTypeRef};
use crate::cf_retained::CFRetained;
use crate::cf_string::CFString;

pub type CFDictionaryRef = *const c_void;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        numValues: CFIndex,
        keyCallBacks: *const c_void,
        valueCallBacks: *const c_void,
    ) -> CFDictionaryRef;
}

pub struct CFDictionary(CFRetained);

impl CFDictionary {
    pub fn as_ptr(&self) -> CFDictionaryRef {
        self.0.as_ptr()
    }

    pub fn from_entries(
        entries: &[(CFString, CFRetained)],
    ) -> Result<Self, EntriesToCFDictionaryError> {
        let count =
            CFIndex::try_from(entries.len()).map_err(|_| EntriesToCFDictionaryError::TooLarge)?;
        let key_ptrs: Vec<_> = entries.iter().map(|(key, _)| key.as_ptr()).collect();
        let value_ptrs: Vec<_> = entries.iter().map(|(_, value)| value.as_ptr()).collect();

        let ptr = unsafe {
            CFDictionaryCreate(
                std::ptr::null(),
                key_ptrs.as_ptr(),
                value_ptrs.as_ptr(),
                count,
                &raw const kCFTypeDictionaryKeyCallBacks,
                &raw const kCFTypeDictionaryValueCallBacks,
            )
        };

        let retained = unsafe { CFRetained::from_raw(ptr) };
        retained.map(Self).ok_or(EntriesToCFDictionaryError::Null)
    }
}

#[derive(Debug)]
pub enum EntriesToCFDictionaryError {
    TooLarge,
    Null,
}

impl fmt::Display for EntriesToCFDictionaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => write!(f, "dictionary entry count exceeds CFIndex::MAX"),
            Self::Null => write!(f, "CFDictionaryCreate returned null"),
        }
    }
}

impl error::Error for EntriesToCFDictionaryError {}
