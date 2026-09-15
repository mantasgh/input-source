use std::ffi::{c_void, c_char, CStr};

type Boolean = u8;
type CFIndex = isize;
type CFDictionaryRef = *const c_void;
type CFTypeRef = *const c_void;
type CFArrayRef = *const c_void;
type CFStringRef = *const c_void;
type CFStringEncoding = u32;
type TISInputSourceRef = *mut c_void;

const kCFStringEncodingUTF8: CFStringEncoding = 0x0800_0100;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
  static kTISPropertyLocalizedName: CFStringRef;
  static kTISPropertyInputSourceID: CFStringRef;
  fn TISCreateInputSourceList(properties: CFDictionaryRef, includeAllInstalled: Boolean) -> CFArrayRef;
  fn TISGetInputSourceProperty(inputSource: TISInputSourceRef, propertyKey: CFStringRef) -> *mut c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
  fn CFArrayGetCount(array: CFArrayRef) -> CFIndex;
  fn CFArrayGetValueAtIndex(array: CFArrayRef, index: CFIndex) -> *const c_void;
  fn CFShow(object: CFTypeRef);
  fn CFRelease(object: CFTypeRef);
  fn CFStringGetLength(string: CFStringRef) -> CFIndex;
  fn CFStringGetMaximumSizeForEncoding(length: CFIndex, encoding: CFStringEncoding) -> CFIndex;
  fn CFStringGetCString(string: CFStringRef, buffer: *mut c_char, bufferSize: CFIndex, encoding: CFStringEncoding) -> Boolean;
}

fn cf_string_to_string(string: CFStringRef) -> Option<String> {
    if string.is_null() {
        println!("CFStringRef is null");
        return None;
    }

    let length = unsafe {
        CFStringGetLength(string)
    };

    println!("length: {length}");

    let maximum_size = unsafe {
        CFStringGetMaximumSizeForEncoding(length, kCFStringEncodingUTF8)
    };

    println!("maximum_size: {maximum_size}");

    if maximum_size < 0 {
        println!("maximum_size is less than 0");
        return None;
    }

    // One additional byte for the terminating null character.
    let size = maximum_size.checked_add(1);
    let size = match size {
        Some(size) => size,
        None => {
            println!("String won't fit into buffer");
            return None;
        }
    };
    let mut buffer = vec![0_u8; size as usize];

    let success = unsafe {
        CFStringGetCString(string, buffer.as_mut_ptr().cast(), size, kCFStringEncodingUTF8)
    };

    println!("success: {success}");

    if success == 0 {
        println!("CFStringGetCString failed");
        return None;
    }

    let c_string = unsafe {
        CStr::from_ptr(buffer.as_ptr().cast())
    };

    let string = c_string.to_string_lossy().into_owned();

    return Some(string)
}

fn main() {
    let list = unsafe {
        TISCreateInputSourceList(std::ptr::null(), 0)
    };

    if list.is_null() {
        println!("TISCreateInputSourceList failed");
        return
    }

    let count = unsafe {
        CFArrayGetCount(list)
    };

    for index in 0..count {
        let value = unsafe {
            CFArrayGetValueAtIndex(list, index)
        };

        if value.is_null() {
            println!("TISInputSourceRef is null");
            continue;
        }

        let input_source = value as TISInputSourceRef;

        let name = unsafe {
            TISGetInputSourceProperty(input_source, kTISPropertyLocalizedName) as CFStringRef
        };

        if name.is_null() {
            println!("name is null");
            continue;
        }

        let name_string = cf_string_to_string(name).unwrap_or(String::from("failed"));
        println!("name: {name_string}");

        let input_source_id = unsafe {
            TISGetInputSourceProperty(input_source, kTISPropertyInputSourceID) as CFStringRef
        };

        if input_source_id.is_null() {
            println!("input_source_id is null");
            continue;
        }
        
        let input_source_id_string = cf_string_to_string(input_source_id).unwrap_or(String::from("failed"));
        println!("input_source_id: {input_source_id_string}");

    }

    unsafe {
        CFRelease(list);
    }
}
