use std::ffi::c_void;
use std::ptr::NonNull;

use crate::cf_base::CFTypeRef;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(object: CFTypeRef);
    fn CFRetain(object: CFTypeRef) -> CFTypeRef;
}

pub struct CFRetained(NonNull<c_void>);

impl CFRetained {
    pub fn as_ptr(&self) -> CFTypeRef {
        self.0.as_ptr().cast_const()
    }

    pub unsafe fn from_raw(ptr: CFTypeRef) -> Option<Self> {
        NonNull::new(ptr.cast_mut()).map(Self)
    }

    pub unsafe fn from_borrowed(ptr: CFTypeRef) -> Option<Self> {
        let ptr = NonNull::new(ptr.cast_mut())?;

        unsafe {
            CFRetain(ptr.as_ptr().cast_const());
        }

        Some(Self(ptr))
    }
}

impl Drop for CFRetained {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.as_ptr());
        }
    }
}
