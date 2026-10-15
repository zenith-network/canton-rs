use std::{error::Error, ffi::OsString, fmt, io, process::ExitStatus};

pub(crate) type Result<T> = std::result::Result<T, Failure>;

#[derive(Debug)]
pub(crate) enum Failure {
    Interrupted,
    Io(io::Error),
    Process(processkit::Error),
    Message(String),
    Command { code: u8, program: OsString },
}

impl Failure {
    pub(crate) fn command(program: OsString, status: ExitStatus) -> Self {
        let code = status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .filter(|code| *code > 0)
            .unwrap_or(1);
        Self::Command { code, program }
    }

    pub(crate) fn exit_code(&self) -> u8 {
        match self {
            Self::Interrupted => 130,
            Self::Command { code, .. } => *code,
            _ => 1,
        }
    }
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<processkit::Error> for Failure {
    fn from(error: processkit::Error) -> Self {
        Self::Process(error)
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interrupted => write!(f, "Interrupted; stopping Canton and test processes."),
            Self::Io(error) => error.fmt(f),
            Self::Process(error) => error.fmt(f),
            Self::Message(message) => message.fmt(f),
            Self::Command { code, program } => {
                write!(
                    f,
                    "{} failed with exit status {code}",
                    program.to_string_lossy()
                )
            }
        }
    }
}

impl Error for Failure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Process(error) => Some(error),
            _ => None,
        }
    }
}

/// Always evaluate cleanup, preserve the primary failure, and report secondary
/// failures rather than letting an early `?` hide them.
pub(crate) fn after_cleanup<T>(outcome: Result<T>, cleanup: Result<()>) -> Result<T> {
    match (outcome, cleanup) {
        (Err(error), Err(cleanup)) => {
            eprintln!("Cleanup failed: {cleanup}");
            Err(error)
        }
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}
