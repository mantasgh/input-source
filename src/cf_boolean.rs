use std::ffi::c_void;

use crate::cf_retained::CFRetained;
use crate::mac_types::Boolean;

pub type CFBooleanRef = *const c_void;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFBooleanGetValue(boolean: CFBooleanRef) -> Boolean;
}

pub struct CFBoolean(CFRetained);

impl CFBoolean {
    pub fn as_ptr(&self) -> CFBooleanRef {
        self.0.as_ptr()
    }

    pub unsafe fn from_borrowed(ptr: CFBooleanRef) -> Option<Self> {
        let retained = unsafe { CFRetained::from_borrowed(ptr) };
        retained.map(Self)
    }
}

impl Into<bool> for CFBoolean {
    fn into(self) -> bool {
        let ptr = self.as_ptr();
        let bool = unsafe { CFBooleanGetValue(ptr) };
        bool != 0
    }
}
