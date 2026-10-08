use std::error;
use std::ffi::c_void;
use std::fmt;

use crate::cf_base::CFIndex;
use crate::cf_retained::CFRetained;

pub type CFArrayRef = *const c_void;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, index: CFIndex) -> *const c_void;
}

pub struct CFArray(CFRetained);

impl CFArray {
    pub fn as_ptr(&self) -> CFArrayRef {
        self.0.as_ptr()
    }

    pub unsafe fn from_raw(ptr: CFArrayRef) -> Option<Self> {
        let retained = unsafe { CFRetained::from_raw(ptr) };
        retained.map(Self)
    }
}

#[derive(Debug)]
pub enum CFArrayToVecError {
    NullArrayElement,
}

impl fmt::Display for CFArrayToVecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CFArrayGetValueAtIndex returned null")
    }
}

impl error::Error for CFArrayToVecError {}

impl TryInto<Vec<CFRetained>> for CFArray {
    type Error = CFArrayToVecError;

    fn try_into(self) -> Result<Vec<CFRetained>, Self::Error> {
        let ptr = self.as_ptr();
        let count = unsafe { CFArrayGetCount(ptr) };

        (0..count)
            .map(|index| {
                let value = unsafe { CFArrayGetValueAtIndex(ptr, index) };
                let retained = unsafe { CFRetained::from_borrowed(value) };
                retained.ok_or(CFArrayToVecError::NullArrayElement)
            })
            .collect()
    }
}
