use std::ffi::OsString;
use std::path::PathBuf;

pub const USAGE: &str = "Usage: input-source <PATH> <INPUT_SOURCE_ID>

Register and enable an installed macOS keyboard layout or input method.

PATH must be inside a supported Library/Keyboard Layouts or Library/Input Methods
directory (system-wide or in your home directory). Files are not copied or downloaded.
Enabling makes the input source available; it does not select it for typing.

Quote paths containing spaces. Use \"$HOME/Library/...\", not \"~/Library/...\".
Use -- before a path beginning with a hyphen.

Options:
  -h, --help  Show this help";

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub path: PathBuf,
    pub input_source_id: String,
}

// None means help was requested. Arguments exclude the executable name.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Option<Args>, &'static str> {
    let mut args = args.into_iter();
    let first = args
        .next()
        .ok_or("missing layout path and input-source ID")?;
    if first == "--help" || first == "-h" {
        return if args.next().is_none() {
            Ok(None)
        } else {
            Err("help does not accept additional arguments")
        };
    }
    let path = if first == "--" {
        args.next()
            .ok_or("missing layout path and input-source ID")?
    } else {
        if first.as_encoded_bytes().starts_with(b"-") {
            return Err("unknown option; use --help for usage or -- before the path");
        }
        first
    };
    if path.is_empty() {
        return Err("layout path must not be empty");
    }
    let input_source_id = args.next().ok_or("missing input-source ID")?;
    if args.next().is_some() {
        return Err("too many arguments; expected a path and an input-source ID");
    }
    let input_source_id = input_source_id
        .into_string()
        .map_err(|_| "input-source ID must be valid UTF-8")?;
    if input_source_id.trim().is_empty() {
        return Err("input-source ID must not be empty");
    }
    Ok(Some(Args {
        path: PathBuf::from(path),
        input_source_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    fn parse_strings(args: &[&str]) -> Result<Option<Args>, &'static str> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn accepts_path_with_spaces_and_id() {
        let path = "/Library/Keyboard Layouts/example.keylayout";
        assert_eq!(
            parse_strings(&[path, "example.id"]),
            Ok(Some(Args {
                path: path.into(),
                input_source_id: "example.id".into()
            }))
        );
    }

    #[test]
    fn accepts_both_help_options() {
        assert_eq!(parse_strings(&["--help"]), Ok(None));
        assert_eq!(parse_strings(&["-h"]), Ok(None));
        assert!(parse_strings(&["--help", "extra"]).is_err());
    }

    #[test]
    fn rejects_missing_extra_and_empty_arguments() {
        for args in [
            vec![],
            vec!["path"],
            vec!["path", "id", "extra"],
            vec!["", "id"],
            vec!["path", ""],
            vec!["path", " "],
            vec!["--"],
        ] {
            assert!(parse_strings(&args).is_err(), "accepted {args:?}");
        }
    }

    #[test]
    fn rejects_unknown_options() {
        assert!(parse_strings(&["--unknown", "id"]).is_err());
    }

    #[test]
    fn accepts_hyphenated_path_after_separator() {
        let args = parse_strings(&["--", "-layout.keylayout", "id"])
            .unwrap()
            .unwrap();
        assert_eq!(args.path, PathBuf::from("-layout.keylayout"));
    }

    #[test]
    fn preserves_non_utf8_paths() {
        let path = OsString::from_vec(b"layout-\xff.keylayout".to_vec());
        let args = parse([path.clone(), OsString::from("id")])
            .unwrap()
            .unwrap();
        assert_eq!(args.path, PathBuf::from(path));
    }

    #[test]
    fn rejects_non_utf8_id() {
        assert!(parse([OsString::from("path"), OsString::from_vec(vec![0xff])]).is_err());
    }
}
