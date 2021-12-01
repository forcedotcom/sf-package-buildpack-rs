use anyhow::anyhow;
use std::process::Output;
use std::{fmt::Display, io::Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub trait Logger {
    /// Display new header section
    fn header(&mut self, msg: impl Display) -> anyhow::Result<()>;
    /// Display an info message
    fn info(&mut self, msg: impl Display) -> anyhow::Result<()>;
    /// Display an error
    fn error(&mut self, header: impl Display, msg: impl Display) -> anyhow::Result<()>;
    /// Display a warning
    fn warning(&mut self, header: impl Display, msg: impl Display) -> anyhow::Result<()>;
    /// Display debug information
    fn debug(&mut self, msg: impl Display) -> anyhow::Result<()>;
    /// Display output from an executed process
    fn output(&mut self, msg: impl Display, output: Output) -> anyhow::Result<()>;
}

/// A logger that uses generics for the implementation of stderr/stdout.
pub struct GenericLogger<T: Write + WriteColor> {
    debug: bool,
    stderr: T,
    stdout: T,
}

/// Salesforce/Heroku Buildpack Logger
pub type BuildLogger = GenericLogger<StandardStream>;

impl BuildLogger {
    /// Create a new logger storing whether debug is set
    pub fn new(debug: bool) -> Self {
        BuildLogger {
            debug,
            stderr: StandardStream::stderr(ColorChoice::Always),
            stdout: StandardStream::stdout(ColorChoice::Always),
        }
    }
}

impl<T: Write + WriteColor> Logger for GenericLogger<T> {
    fn header(&mut self, msg: impl Display) -> anyhow::Result<()> {
        Ok(header(&mut self.stdout, msg)?)
    }

    fn info(&mut self, msg: impl Display) -> anyhow::Result<()> {
        Ok(info(&mut self.stdout, msg)?)
    }

    fn error(&mut self, header: impl Display, msg: impl Display) -> anyhow::Result<()> {
        Ok(error(&mut self.stderr, header, msg)?)
    }

    fn warning(&mut self, header: impl Display, msg: impl Display) -> anyhow::Result<()> {
        Ok(warning(&mut self.stdout, header, msg)?)
    }

    fn debug(&mut self, msg: impl Display) -> anyhow::Result<()> {
        Ok(debug(&mut self.stdout, msg, self.debug)?)
    }

    fn output(&mut self, header: impl Display, result: Output) -> anyhow::Result<()> {
        Ok(output(&mut self.stdout, header, result, self.debug)?)
    }
}

pub fn header(stdout: &mut impl WriteColor, msg: impl Display) -> anyhow::Result<()> {
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))?;
    writeln!(stdout, "\n[{}]", msg)?;
    stdout.reset()?;
    stdout.flush()?;

    Ok(())
}

pub fn info(stdout: &mut impl WriteColor, msg: impl Display) -> anyhow::Result<()> {
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
    writeln!(stdout, "[INFO] {}", msg)?;
    stdout.flush()?;

    Ok(())
}

pub fn error(
    stderr: &mut impl WriteColor,
    header: impl Display,
    msg: impl Display,
) -> anyhow::Result<()> {
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
    writeln!(stderr, "\n[ERROR: {}]", header)?;
    stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    writeln!(stderr, "{}", msg)?;
    stderr.reset()?;
    stderr.flush()?;

    Err(anyhow!(format!("{}", header)))
}

pub fn debug(stdout: &mut impl WriteColor, msg: impl Display, debug: bool) -> anyhow::Result<()> {
    if debug {
        writeln!(stdout, "[DEBUG] {}", msg)?;
        stdout.flush()?;
    }

    Ok(())
}

pub fn output(
    stdout: &mut impl WriteColor,
    header: impl Display,
    output: Output,
    debug: bool,
) -> anyhow::Result<()> {
    if debug {
        let status = output.status;
        if !&output.stdout.is_empty() {
            writeln!(stdout, "---> {}", String::from_utf8_lossy(&output.stdout))?;
        }
        if !&output.stderr.is_empty() {
            if status.success() {
                // Yes, some sfdx commands like force:source:push decided to output progress to stderr.
                writeln!(stdout, "---> {}", String::from_utf8_lossy(&output.stderr))?;
            } else {
                error(
                    stdout,
                    format!("---> Failed {}", header),
                    format!("---> {}", String::from_utf8_lossy(&output.stderr)),
                )?;
            }
        }
    }

    Ok(())
}

pub fn warning(
    stdout: &mut impl WriteColor,
    header: impl Display,
    msg: impl Display,
) -> anyhow::Result<()> {
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
    writeln!(stdout, "\n[WARNING: {}]", header)?;
    stdout.flush()?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    writeln!(stdout, "{}", msg)?;
    stdout.reset()?;
    stdout.flush()?;

    Ok(())
}
