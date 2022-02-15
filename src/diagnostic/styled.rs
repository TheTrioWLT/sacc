use std::slice::Iter;

use termcolor::ColorSpec;

use super::{Level, PROMPT_COLOR};

#[derive(Debug, Clone)]
pub struct StyledString {
    pub text: String,
    pub style: Style,
}

impl StyledString {
    /// Creates a new StyledString
    pub fn new(text: String, style: Style) -> Self {
        Self { text, style }
    }
}

/// A style that a part of a diagnostic can have
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Style {
    /// level: **main header message**
    MainHeaderMsg,
    /// Plain text
    NoStyle,
    /// The line number of a specific code snippet
    LineNumber,
    /// The line that makes up a diagnostic message's separation between the line numbers and code
    LineAndColumn,
    /// The style associated with a specific diagnostic level
    Level(Level),
}

impl Style {
    /// Converts a Style into a ColorSpec for colored output
    pub fn to_spec(&self) -> ColorSpec {
        match self {
            Style::MainHeaderMsg => {
                let mut spec = ColorSpec::new();
                // If we want the plain white
                //
                // spec.set_fg(Some(PLAIN_WHITE));
                spec.set_bold(true);

                spec
            }
            Style::NoStyle => ColorSpec::new(),
            Style::LineNumber | Style::LineAndColumn => {
                let mut spec = ColorSpec::new();
                spec.set_fg(Some(PROMPT_COLOR));
                spec.set_intense(true);
                spec.set_bold(true);

                spec
            }
            Style::Level(l) => l.color(),
        }
    }
}

/// A buffer consisting of the various styled parts of a diagnostic message
#[derive(Debug)]
pub struct StyledBuffer {
    parts: Vec<StyledString>,
}

impl StyledBuffer {
    /// Creates a new StyledBuffer
    pub fn new() -> StyledBuffer {
        Self { parts: Vec::new() }
    }

    /// Inserts a string into the StyledBuffer
    pub fn puts(&mut self, s: StyledString) {
        self.parts.push(s);
    }

    pub fn iter(&self) -> Iter<StyledString> {
        self.parts.iter()
    }
}
