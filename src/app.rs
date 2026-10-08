use std::convert::Infallible;
use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::carbon::{
    self, EnableInputSourceError, InputSourceFilter, InputSourceListError, PropertyError,
    RegisterInputSourceError, TISInputSource,
};
use crate::cf_string::CFStringToStringError;

pub trait InputSourceBackend {
    type Source;

    fn list(
        &mut self,
        filters: Option<Vec<InputSourceFilter>>,
        include_all_installed: bool,
    ) -> Result<Vec<Self::Source>, InputSourceListError>;
    fn register(&mut self, path: &Path) -> Result<(), RegisterInputSourceError>;
    fn name(
        &mut self,
        source: &Self::Source,
    ) -> Result<String, PropertyError<CFStringToStringError>>;
    fn id(&mut self, source: &Self::Source)
    -> Result<String, PropertyError<CFStringToStringError>>;
    fn is_enabled(&mut self, source: &Self::Source) -> Result<bool, PropertyError<Infallible>>;
    fn enable(&mut self, source: &Self::Source) -> Result<(), EnableInputSourceError>;
}

pub struct CarbonBackend;

impl InputSourceBackend for CarbonBackend {
    type Source = TISInputSource;

    fn list(
        &mut self,
        filters: Option<Vec<InputSourceFilter>>,
        include_all_installed: bool,
    ) -> Result<Vec<Self::Source>, InputSourceListError> {
        carbon::create_input_source_list(filters, include_all_installed)
    }

    fn register(&mut self, path: &Path) -> Result<(), RegisterInputSourceError> {
        carbon::register_input_source(path)
    }

    fn name(
        &mut self,
        source: &Self::Source,
    ) -> Result<String, PropertyError<CFStringToStringError>> {
        source.get_localized_name()
    }

    fn id(
        &mut self,
        source: &Self::Source,
    ) -> Result<String, PropertyError<CFStringToStringError>> {
        source.get_input_source_id()
    }

    fn is_enabled(&mut self, source: &Self::Source) -> Result<bool, PropertyError<Infallible>> {
        source.get_input_source_enabled()
    }

    fn enable(&mut self, source: &Self::Source) -> Result<(), EnableInputSourceError> {
        source.enable_input_source()
    }
}

#[derive(Debug)]
pub enum AppError {
    List(InputSourceListError),
    Register(RegisterInputSourceError),
    NotFound(String),
    Name(PropertyError<CFStringToStringError>),
    ID(PropertyError<CFStringToStringError>),
    Enabled(PropertyError<Infallible>),
    Enable(EnableInputSourceError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::List(_) => write!(f, "failed to create input source list"),
            Self::Register(_) => write!(f, "failed to register input source"),
            Self::NotFound(id) => write!(f, "input source {id:?} was not found after registration"),
            Self::Name(_) => write!(f, "failed to get localized name"),
            Self::ID(_) => write!(f, "failed to get input source ID"),
            Self::Enabled(_) => write!(f, "failed to get input source enabled state"),
            Self::Enable(_) => write!(f, "failed to enable input source"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::List(err) => Some(err),
            Self::Register(err) => Some(err),
            Self::Name(err) | Self::ID(err) => Some(err),
            Self::Enabled(err) => Some(err),
            Self::Enable(err) => Some(err),
            Self::NotFound(_) => None,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Name(String),
    InputSourceID(String),
    Enabled(bool),
}

#[derive(Debug, PartialEq, Eq)]
pub enum EnableOutcome {
    AlreadyEnabled,
    Enabled,
}

pub fn run(
    backend: &mut impl InputSourceBackend,
    path: &Path,
    input_source_id: &str,
    mut report: impl FnMut(Event),
) -> Result<EnableOutcome, AppError> {
    backend.register(path).map_err(AppError::Register)?;

    // Verify that the requested ID is discoverable after registration.
    let filters = vec![InputSourceFilter::InputSourceID(input_source_id.to_owned())];
    let sources = backend.list(Some(filters), true).map_err(AppError::List)?;
    let source = sources
        .first()
        .ok_or_else(|| AppError::NotFound(input_source_id.to_owned()))?;

    report(Event::Name(backend.name(source).map_err(AppError::Name)?));
    report(Event::InputSourceID(
        backend.id(source).map_err(AppError::ID)?,
    ));

    let enabled = backend.is_enabled(source).map_err(AppError::Enabled)?;
    report(Event::Enabled(enabled));
    if enabled {
        return Ok(EnableOutcome::AlreadyEnabled);
    }

    backend.enable(source).map_err(AppError::Enable)?;
    Ok(EnableOutcome::Enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "/not-a-real-installation/example.keylayout";
    const ID: &str = "test.input-source";
    const NAME: &str = "Test input source";

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Operation {
        Register,
        List,
        Name,
        ID,
        IsEnabled,
        Enable,
    }

    // No CF objects, filesystem access, or native calls are used by this backend.
    struct FakeBackend {
        calls: Vec<Operation>,
        fail_at: Option<Operation>,
        sources: Vec<usize>,
        enabled: bool,
    }

    impl Default for FakeBackend {
        fn default() -> Self {
            Self {
                calls: Vec::new(),
                fail_at: None,
                sources: vec![42],
                enabled: false,
            }
        }
    }

    impl FakeBackend {
        fn call(&mut self, operation: Operation) -> bool {
            self.calls.push(operation);
            self.fail_at == Some(operation)
        }
    }

    impl InputSourceBackend for FakeBackend {
        type Source = usize;

        fn register(&mut self, path: &Path) -> Result<(), RegisterInputSourceError> {
            assert_eq!(path, Path::new(PATH));
            if self.call(Operation::Register) {
                Err(RegisterInputSourceError::Status(-50))
            } else {
                Ok(())
            }
        }

        fn list(
            &mut self,
            filters: Option<Vec<InputSourceFilter>>,
            include_all_installed: bool,
        ) -> Result<Vec<Self::Source>, InputSourceListError> {
            assert!(
                include_all_installed,
                "lookup must include disabled sources"
            );
            let filters = filters.expect("lookup must filter by the requested ID");
            assert!(matches!(
                filters.as_slice(),
                [InputSourceFilter::InputSourceID(id)] if id == ID
            ));
            if self.call(Operation::List) {
                Err(InputSourceListError::Null)
            } else {
                Ok(self.sources.clone())
            }
        }

        fn name(
            &mut self,
            source: &Self::Source,
        ) -> Result<String, PropertyError<CFStringToStringError>> {
            assert_eq!(*source, 42);
            if self.call(Operation::Name) {
                Err(PropertyError::Conversion(
                    CFStringToStringError::InvalidSize,
                ))
            } else {
                Ok(NAME.to_owned())
            }
        }

        fn id(
            &mut self,
            source: &Self::Source,
        ) -> Result<String, PropertyError<CFStringToStringError>> {
            assert_eq!(*source, 42);
            if self.call(Operation::ID) {
                Err(PropertyError::Null)
            } else {
                Ok(ID.to_owned())
            }
        }

        fn is_enabled(&mut self, source: &Self::Source) -> Result<bool, PropertyError<Infallible>> {
            assert_eq!(*source, 42);
            if self.call(Operation::IsEnabled) {
                Err(PropertyError::Null)
            } else {
                Ok(self.enabled)
            }
        }

        fn enable(&mut self, source: &Self::Source) -> Result<(), EnableInputSourceError> {
            assert_eq!(*source, 42);
            if self.call(Operation::Enable) {
                Err(EnableInputSourceError::Status(-50))
            } else {
                self.enabled = true;
                Ok(())
            }
        }
    }

    const CALL_ORDER: [Operation; 6] = [
        Operation::Register,
        Operation::List,
        Operation::Name,
        Operation::ID,
        Operation::IsEnabled,
        Operation::Enable,
    ];

    fn execute(backend: &mut FakeBackend) -> (Result<EnableOutcome, AppError>, Vec<Event>) {
        let mut events = Vec::new();
        let result = run(backend, Path::new(PATH), ID, |event| events.push(event));
        (result, events)
    }

    fn assert_events(events: &[Event], enabled: bool) {
        for (index, event) in events.iter().enumerate() {
            match (index, event) {
                (0, Event::Name(name)) => assert_eq!(name, NAME),
                (1, Event::InputSourceID(id)) => assert_eq!(id, ID),
                (2, Event::Enabled(value)) => assert_eq!(*value, enabled),
                _ => panic!("unexpected event at index {index}: {event:?}"),
            }
        }
    }

    #[test]
    fn registers_finds_and_enables_disabled_source() {
        let mut backend = FakeBackend::default();
        let (result, events) = execute(&mut backend);

        assert_eq!(result.unwrap(), EnableOutcome::Enabled);
        assert_eq!(backend.calls, CALL_ORDER);
        assert!(backend.enabled);
        assert_eq!(events.len(), 3);
        assert_events(&events, false);
    }

    #[test]
    fn already_enabled_source_is_not_enabled_again() {
        let mut backend = FakeBackend {
            enabled: true,
            ..FakeBackend::default()
        };
        let (result, events) = execute(&mut backend);

        assert_eq!(result.unwrap(), EnableOutcome::AlreadyEnabled);
        assert_eq!(backend.calls, CALL_ORDER[..5]);
        assert_eq!(events.len(), 3);
        assert_events(&events, true);
    }

    #[test]
    fn missing_source_returns_requested_id_without_reading_properties() {
        let mut backend = FakeBackend {
            sources: Vec::new(),
            ..FakeBackend::default()
        };
        let (result, events) = execute(&mut backend);
        let error = result.unwrap_err();

        assert!(matches!(&error, AppError::NotFound(id) if id == ID));
        assert!(error.source().is_none());
        assert_eq!(backend.calls, CALL_ORDER[..2]);
        assert!(events.is_empty());
    }

    #[test]
    fn multiple_matches_use_only_first_source() {
        let mut backend = FakeBackend {
            sources: vec![42, 99],
            ..FakeBackend::default()
        };
        let (result, _) = execute(&mut backend);

        assert_eq!(result.unwrap(), EnableOutcome::Enabled);
        assert_eq!(backend.calls, CALL_ORDER);
    }

    fn assert_failure(operation: Operation) {
        let mut backend = FakeBackend {
            fail_at: Some(operation),
            ..FakeBackend::default()
        };
        let (result, events) = execute(&mut backend);
        let error = result.unwrap_err();
        let cause = error.source().expect("backend error must be preserved");

        let event_count = match (operation, &error) {
            (Operation::Register, AppError::Register(RegisterInputSourceError::Status(-50))) => {
                assert!(cause.is::<RegisterInputSourceError>());
                0
            }
            (Operation::List, AppError::List(InputSourceListError::Null)) => {
                assert!(cause.is::<InputSourceListError>());
                0
            }
            (
                Operation::Name,
                AppError::Name(PropertyError::Conversion(CFStringToStringError::InvalidSize)),
            ) => {
                assert!(matches!(
                    cause
                        .source()
                        .unwrap()
                        .downcast_ref::<CFStringToStringError>(),
                    Some(CFStringToStringError::InvalidSize)
                ));
                0
            }
            (Operation::ID, AppError::ID(PropertyError::Null)) => {
                assert!(cause.is::<PropertyError<CFStringToStringError>>());
                1
            }
            (Operation::IsEnabled, AppError::Enabled(PropertyError::Null)) => {
                assert!(cause.is::<PropertyError<Infallible>>());
                2
            }
            (Operation::Enable, AppError::Enable(EnableInputSourceError::Status(-50))) => {
                assert!(cause.is::<EnableInputSourceError>());
                3
            }
            _ => panic!("unexpected error for {operation:?}: {error:?}"),
        };

        let last_call = CALL_ORDER
            .iter()
            .position(|call| *call == operation)
            .unwrap();
        assert_eq!(backend.calls, CALL_ORDER[..=last_call]);
        assert!(!backend.enabled);
        assert_eq!(events.len(), event_count);
        assert_events(&events, false);
    }

    #[test]
    fn registration_failure_stops_workflow() {
        assert_failure(Operation::Register);
    }

    #[test]
    fn list_failure_stops_workflow() {
        assert_failure(Operation::List);
    }

    #[test]
    fn name_failure_stops_workflow_and_preserves_nested_cause() {
        assert_failure(Operation::Name);
    }

    #[test]
    fn id_failure_stops_workflow() {
        assert_failure(Operation::ID);
    }

    #[test]
    fn enabled_state_failure_stops_workflow() {
        assert_failure(Operation::IsEnabled);
    }

    #[test]
    fn enable_failure_is_propagated() {
        assert_failure(Operation::Enable);
    }
}
