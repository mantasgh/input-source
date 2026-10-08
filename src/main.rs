use std::error::Error;
use std::process::ExitCode;

mod app;
mod carbon;
mod cf_array;
mod cf_base;
mod cf_boolean;
mod cf_dictionary;
mod cf_retained;
mod cf_string;
mod cf_url;
mod cli;
mod mac_types;

use app::{CarbonBackend, EnableOutcome, Event};

fn main() -> ExitCode {
    let args = match cli::parse(std::env::args_os().skip(1)) {
        Ok(Some(args)) => args,
        Ok(None) => {
            println!("{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("Error: {error}\n\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    let path = &args.path;
    let input_source_id = &args.input_source_id;
    let mut backend = CarbonBackend;

    println!("Registering from {}...", path.display());

    let result = app::run(&mut backend, path, input_source_id, |event| match event {
        Event::Name(name) => println!("Found: {name}"),
        Event::InputSourceID(_id) => {}
        Event::Enabled(false) => println!("Enabling..."),
        Event::Enabled(true) => {}
    });

    match result {
        Ok(EnableOutcome::AlreadyEnabled) => {
            println!("Already enabled.");
            ExitCode::SUCCESS
        }
        Ok(EnableOutcome::Enabled) => {
            println!("Enable request submitted. If macOS asks for confirmation, click Allow.");
            ExitCode::SUCCESS
        }
        Err(err) => {
            print_error(&err);
            ExitCode::FAILURE
        }
    }
}

fn print_error(error: &dyn Error) {
    eprintln!("Error: {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        eprintln!("  Caused by: {cause}");
        source = cause.source();
    }
}
