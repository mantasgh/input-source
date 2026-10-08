use std::collections::HashSet;
use std::convert::Infallible;
use std::env;
use std::error::Error;
use std::ffi::c_void;
use std::fmt;
use std::path::Path;

use crate::cf_array::{CFArray, CFArrayRef, CFArrayToVecError};
use crate::cf_boolean::{CFBoolean, CFBooleanRef};
use crate::cf_dictionary::{CFDictionary, CFDictionaryRef, EntriesToCFDictionaryError};
use crate::cf_retained::CFRetained;
use crate::cf_string::{CFString, CFStringRef, CFStringToStringError, StrToCFStringError};
use crate::cf_url::{CFURL, CFURLRef, PathToCFURLError};
use crate::mac_types::{Boolean, OSStatus};

pub type TISInputSourceRef = *mut c_void;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyLocalizedName: CFStringRef;
    static kTISPropertyInputSourceID: CFStringRef;
    static kTISPropertyInputSourceIsEnabled: CFStringRef;
    fn TISCreateInputSourceList(
        properties: CFDictionaryRef,
        includeAllInstalled: Boolean,
    ) -> CFArrayRef;
    fn TISGetInputSourceProperty(
        inputSource: TISInputSourceRef,
        propertyKey: CFStringRef,
    ) -> *mut c_void;
    fn TISRegisterInputSource(location: CFURLRef) -> OSStatus;
    fn TISEnableInputSource(inputSource: TISInputSourceRef) -> OSStatus;
}

#[derive(PartialEq, Eq, Hash)]
pub enum InputSourceFilter {
    InputSourceID(String),
}

#[derive(Debug)]
pub enum InputSourceListError {
    Null,
    NullPropertyKey,
    StringConversion(StrToCFStringError),
    DictionaryConversion(EntriesToCFDictionaryError),
    ArrayConversion(CFArrayToVecError),
}

impl fmt::Display for InputSourceListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "TISCreateInputSourceList returned null"),
            Self::NullPropertyKey => write!(f, "input source property key constant is null"),
            Self::StringConversion(_) => {
                write!(f, "failed to convert input source filter value to CFString")
            }
            Self::DictionaryConversion(_) => {
                write!(f, "failed to create input source filter dictionary")
            }
            Self::ArrayConversion(_) => write!(f, "failed to convert input source array to Vec"),
        }
    }
}

impl Error for InputSourceListError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::StringConversion(err) => Some(err),
            Self::DictionaryConversion(err) => Some(err),
            Self::ArrayConversion(err) => Some(err),
            Self::Null | Self::NullPropertyKey => None,
        }
    }
}

pub fn create_input_source_list(
    filters: Option<Vec<InputSourceFilter>>,
    include_all_installed: bool,
) -> Result<Vec<TISInputSource>, InputSourceListError> {
    let include_all_installed = if include_all_installed { 1 } else { 0 };

    let dictionary = if let Some(filters) = filters {
        let mut entries = Vec::with_capacity(filters.len());
        let mut seen = HashSet::new();
        // Work backwards so superseded values are skipped before conversion.
        for filter in filters.into_iter().rev() {
            if !seen.insert(std::mem::discriminant(&filter)) {
                continue;
            }
            let (key, value): (CFStringRef, CFRetained) = match filter {
                InputSourceFilter::InputSourceID(value) => (
                    unsafe { kTISPropertyInputSourceID },
                    CFString::try_from(value.as_str())
                        .map_err(InputSourceListError::StringConversion)?
                        .into(),
                ),
            };
            // These property-key constants are borrowed CFStrings.
            let key = unsafe { CFString::from_borrowed(key) }
                .ok_or(InputSourceListError::NullPropertyKey)?;
            entries.push((key, value));
        }

        Some(
            CFDictionary::from_entries(&entries)
                .map_err(InputSourceListError::DictionaryConversion)?,
        )
    } else {
        None
    };
    let properties_ptr = if let Some(dictionary) = dictionary.as_ref() {
        dictionary.as_ptr()
    } else {
        std::ptr::null()
    };

    let list = unsafe { TISCreateInputSourceList(properties_ptr, include_all_installed) };

    let list = unsafe { CFArray::from_raw(list) };
    let list = list.ok_or(InputSourceListError::Null)?;

    let list: Vec<CFRetained> = list
        .try_into()
        .map_err(InputSourceListError::ArrayConversion)?;

    Ok(list
        .into_iter()
        .map(|item| unsafe { TISInputSource::from_retained(item) })
        .collect())
}

#[derive(Debug)]
pub enum RegisterInputSourceError {
    FileSystem(std::io::Error),
    HomeDirectoryUnavailable,
    InvalidFileType,
    InvalidLocation,
    Path(PathToCFURLError),
    Status(OSStatus),
}

impl fmt::Display for RegisterInputSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileSystem(_) => write!(
                f,
                "failed to inspect input source path or installation locations"
            ),
            Self::HomeDirectoryUnavailable => {
                write!(f, "could not determine the user's home directory")
            }
            Self::InvalidFileType => {
                write!(f, "input source path is not a regular file or directory")
            }
            Self::InvalidLocation => write!(
                f,
                "input source is not in a supported installation location for its file type"
            ),
            Self::Path(_) => write!(f, "failed to convert input source path to CFURL"),
            Self::Status(status) => {
                write!(f, "TISRegisterInputSource failed with OSStatus {status}")
            }
        }
    }
}

impl Error for RegisterInputSourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::FileSystem(err) => Some(err),
            Self::Path(err) => Some(err),
            Self::HomeDirectoryUnavailable
            | Self::InvalidFileType
            | Self::InvalidLocation
            | Self::Status(_) => None,
        }
    }
}

pub fn register_input_source(path: &Path) -> Result<(), RegisterInputSourceError> {
    let canonical_path = path
        .canonicalize()
        .map_err(RegisterInputSourceError::FileSystem)?;
    let metadata = canonical_path
        .metadata()
        .map_err(RegisterInputSourceError::FileSystem)?;

    if !metadata.is_file() && !metadata.is_dir() {
        return Err(RegisterInputSourceError::InvalidFileType);
    }

    // Keyboard layouts may be files or bundles; input methods must be bundles.
    let home = env::home_dir();
    let mut locations = vec![
        (Path::new("/Library/Keyboard Layouts").to_path_buf(), false),
        (Path::new("/Library/Input Methods").to_path_buf(), true),
    ];
    if let Some(home) = &home {
        locations.push((home.join("Library/Keyboard Layouts"), false));
        locations.push((home.join("Library/Input Methods"), true));
    }

    let mut allowed = false;
    let mut location_error = None;
    for (location, requires_directory) in locations {
        if requires_directory && !metadata.is_dir() {
            continue;
        }
        let location = match location.canonicalize() {
            Ok(location) => location,
            // Installation directories need not all exist.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                location_error = Some(err);
                continue;
            }
        };
        if canonical_path != location && canonical_path.starts_with(&location) {
            allowed = true;
            break;
        }
    }

    if !allowed {
        if let Some(err) = location_error {
            return Err(RegisterInputSourceError::FileSystem(err));
        }
        if home.is_none() {
            return Err(RegisterInputSourceError::HomeDirectoryUnavailable);
        }
        return Err(RegisterInputSourceError::InvalidLocation);
    }

    let url = CFURL::from_path(&canonical_path, metadata.is_dir())
        .map_err(RegisterInputSourceError::Path)?;

    let status = unsafe { TISRegisterInputSource(url.as_ptr()) };

    if status != 0 {
        return Err(RegisterInputSourceError::Status(status));
    }

    Ok(())
}

#[derive(Debug)]
pub enum EnableInputSourceError {
    Status(OSStatus),
}

impl fmt::Display for EnableInputSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Status(status) => write!(f, "TISEnableInputSource failed with OSStatus {status}"),
        }
    }
}

impl Error for EnableInputSourceError {}

#[derive(Debug)]
pub enum PropertyError<E> {
    Null,
    Conversion(E),
}

impl<E> fmt::Display for PropertyError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "TISGetInputSourceProperty returned null"),
            Self::Conversion(_) => write!(f, "failed to convert input source property value"),
        }
    }
}

impl<E: Error + 'static> Error for PropertyError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Null => None,
            Self::Conversion(err) => Some(err),
        }
    }
}

type StringPropertyError = PropertyError<CFStringToStringError>;
type BoolPropertyError = PropertyError<Infallible>;

unsafe fn string_property(ptr: CFStringRef) -> Result<String, StringPropertyError> {
    let cf_string = unsafe { CFString::from_borrowed(ptr) };
    let cf_string = cf_string.ok_or(PropertyError::Null)?;
    cf_string.try_into().map_err(PropertyError::Conversion)
}

unsafe fn bool_property(ptr: CFBooleanRef) -> Result<bool, BoolPropertyError> {
    let cf_boolean = unsafe { CFBoolean::from_borrowed(ptr) };
    let cf_boolean = cf_boolean.ok_or(PropertyError::Null)?;
    Ok(cf_boolean.into())
}

pub struct TISInputSource(CFRetained);

impl TISInputSource {
    pub fn as_ptr(&self) -> TISInputSourceRef {
        self.0.as_ptr().cast_mut()
    }

    pub unsafe fn from_retained(retained: CFRetained) -> Self {
        Self(retained)
    }

    pub fn get_localized_name(&self) -> Result<String, StringPropertyError> {
        let name = unsafe {
            TISGetInputSourceProperty(self.as_ptr(), kTISPropertyLocalizedName) as CFStringRef
        };
        unsafe { string_property(name) }
    }

    pub fn get_input_source_id(&self) -> Result<String, StringPropertyError> {
        let input_source_id = unsafe {
            TISGetInputSourceProperty(self.as_ptr(), kTISPropertyInputSourceID) as CFStringRef
        };
        unsafe { string_property(input_source_id) }
    }

    pub fn get_input_source_enabled(&self) -> Result<bool, BoolPropertyError> {
        let enabled = unsafe {
            TISGetInputSourceProperty(self.as_ptr(), kTISPropertyInputSourceIsEnabled)
                as CFBooleanRef
        };
        unsafe { bool_property(enabled) }
    }

    pub fn enable_input_source(&self) -> Result<(), EnableInputSourceError> {
        let status = unsafe { TISEnableInputSource(self.as_ptr()) };

        if status != 0 {
            return Err(EnableInputSourceError::Status(status));
        }

        Ok(())
    }
}
