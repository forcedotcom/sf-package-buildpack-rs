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
    prefix: bool,
    stderr: T,
    stdout: T,
}

/// Salesforce/Heroku Buildpack Logger
pub type BuildLogger = GenericLogger<StandardStream>;

impl BuildLogger {
    /// Create a new logger storing whether debug is set
    pub fn new(debug: bool, prefix: bool) -> Self {
        BuildLogger {
            debug,
            prefix,
            stderr: StandardStream::stderr(ColorChoice::Always),
            stdout: StandardStream::stdout(ColorChoice::Always),
        }
    }
}

impl<T: Write + WriteColor> Logger for GenericLogger<T> {
    fn header(&mut self, msg: impl Display) -> anyhow::Result<()> {
        if self.prefix {
            self.stdout
                .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))?;
            writeln!(self.stdout, "\n[{}]", msg)?;
            self.stdout.reset()?;
            self.stdout.flush()?;
        }
        Ok(())
    }

    fn info(&mut self, msg: impl Display) -> anyhow::Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;
        if self.prefix {
            write!(self.stdout, "[INFO] ")?;
        }

        writeln!(self.stdout, "{}", msg)?;
        self.stdout.flush()?;
        Ok(())
    }

    fn error(&mut self, header: impl Display, msg: impl Display) -> anyhow::Result<()> {
        self.stderr
            .set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        if self.prefix {
            writeln!(self.stderr, "[ERROR] {}", header)?;
        } else {
            writeln!(self.stderr, "{}", header)?;
        }
        self.stderr
            .set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
        writeln!(self.stderr, "{}", msg)?;
        self.stderr.reset()?;
        self.stderr.flush()?;

        Err(anyhow!(format!("{}", header)))
    }

    fn warning(&mut self, header: impl Display, msg: impl Display) -> anyhow::Result<()> {
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;
        if self.prefix {
            writeln!(self.stdout, "[WARNING: {}]", header)?;
        } else {
            writeln!(self.stdout, "{}", header)?;
        }
        self.stdout.flush()?;
        self.stdout
            .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
        writeln!(self.stdout, "{}", msg)?;
        self.stdout.reset()?;
        self.stdout.flush()?;
        Ok(())
    }

    fn debug(&mut self, msg: impl Display) -> anyhow::Result<()> {
        if self.debug {
            if self.prefix {
                write!(self.stdout, "[DEBUG] ")?;
            }
            writeln!(self.stdout, "{}", msg)?;
            self.stdout.flush()?;
        }
        Ok(())
    }

    fn output(&mut self, header: impl Display, result: Output) -> anyhow::Result<()> {
        if self.debug {
            let status = result.status;
            if status.success() {
                if !&result.stdout.is_empty() {
                    writeln!(
                        self.stdout,
                        "---> {}",
                        String::from_utf8_lossy(&result.stdout)
                    )?;
                }
            } else {
                self.error(
                    format!("---> Failed {}", header),
                    format!("---> {}", String::from_utf8_lossy(&result.stderr)),
                )?;
            }
        }
        Ok(())
    }
}
