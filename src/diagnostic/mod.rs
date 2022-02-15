use std::rc::Rc;

use termcolor::{Color, ColorSpec};

use self::styled::{Style, StyledString};

pub mod emitter;
mod handler;
mod source_manager;
pub mod styled;
pub use handler::*;
pub use source_manager::*;
pub mod session;

// All of the different colors that a diagnostic can use
static WARNING_COLOR: Color = Color::Yellow;
static ERROR_COLOR: Color = Color::Red;
static NOTE_COLOR: Color = Color::Green;
static HELP_COLOR: Color = Color::Cyan;
// static PLAIN_WHITE: Color = Color::Rgb(255, 255, 255);
static PROMPT_COLOR: Color = Color::Blue;

pub struct DiagnosticBuilder<'a> {
    diagnostic: Diagnostic,
    handler: &'a Handler,
}

impl<'a> DiagnosticBuilder<'a> {
    /// For internal use only, creates a new DiagnosticBuilder. For clients, the struct_* methods
    /// on a Session or Handler should be used instead.
    pub(crate) fn new(handler: &'a Handler, level: Level, message: String) -> Self {
        let diagnostic = Diagnostic {
            level,
            message,
            primary: None,
            spans: Vec::new(),
            children: Vec::new(),
        };

        Self {
            diagnostic,
            handler,
        }
    }

    pub fn set_primary_span(&mut self, span: Span) -> &mut Self {
        self.diagnostic.primary = Some(span);

        self
    }

    pub fn span_label(&mut self, span: Span, label: String) -> &mut Self {
        self.diagnostic.spans.push((span, label));

        self
    }

    /// Adds a note message to the diagnostic
    pub fn note(&mut self, message: String) -> &mut Self {
        let subd = SubDiagnostic::new(Level::Note, message, None);
        self.diagnostic.children.push(subd);

        self
    }

    /// Adds a help message to the diagnostic
    pub fn help(&mut self, message: String) -> &mut Self {
        let subd = SubDiagnostic::new(Level::Help, message, None);
        self.diagnostic.children.push(subd);

        self
    }

    /// Queues this diagnostic to be emitted by the inner Handler/Emitter
    pub fn emit(&mut self) {
        if self.diagnostic.level == Level::Warning {
            self.handler.warn(self.diagnostic.clone());
        } else {
            self.handler.error(self.diagnostic.clone());
        }

        // Mark this as cancelled so that it can be safely dropped
        self.cancel();
    }

    /// Sets this DiagnosticBuilder as cancelled, meaning that it is safe to be dropped
    pub fn cancel(&mut self) {
        self.diagnostic.level = Level::Cancelled;
    }

    /// Returns true if this was cancelled, false otherwise
    pub fn cancelled(&self) -> bool {
        self.diagnostic.level == Level::Cancelled
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: Level,
    pub message: String,
    pub primary: Option<Span>,
    pub spans: Vec<(Span, String)>,
    pub children: Vec<SubDiagnostic>,
}

impl<'a> Drop for DiagnosticBuilder<'a> {
    fn drop(&mut self) {
        // DiagnosticBuilders are sort of bombs if dropped. This had better either be emitted, or
        // cancelled. If not, we emit a bug error.
        if !self.cancelled() {
            let mut db = DiagnosticBuilder::new(
                self.handler,
                Level::Bug,
                "the following error was constructed but not emitted".to_string(),
            );

            db.emit();
            self.emit();
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubDiagnostic {
    pub level: Level,
    pub message: String,
    pub span: Option<Span>,
}

impl SubDiagnostic {
    pub fn new(level: Level, message: String, span: Option<Span>) -> Self {
        Self {
            level,
            message,
            span,
        }
    }
}

/// A source location broken down into the file, the line, and the column. This is useful for
/// showing diagnostics
#[derive(Debug, Clone)]
pub struct Loc {
    /// The file that this location refers to
    pub file: Rc<SourceFile>,
    /// The line number, starting at line 0!
    pub line: usize,
    /// The column number
    pub col: usize,
}

impl Loc {
    pub fn new(file: Rc<SourceFile>, line: usize, col: usize) -> Self {
        Self { file, line, col }
    }
}

/// A Span is what Diagnostics use to display pieces of code. These can be turned into snippets
/// which actually contain the sourcecode that these Spans point to so that the Diagnostic can be
/// emitted.
#[derive(Debug, Clone, Copy)]
pub struct Span {
    /// The start index into the source String
    pub start: usize,
    /// The end index into the source String
    pub end: usize,
    /// The index into the SourceManager's SourceFile Vec
    pub source: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, source: usize) -> Self {
        Self { start, end, source }
    }
}

/// Represents the level of diagnostic
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Level {
    /// An internal bug in the compiler
    Bug,
    /// A general error during the normal compilation process
    Error,
    /// A warning
    Warning,
    /// A helpful note
    Note,
    /// A suggestion
    Help,
    /// A specific type that represents a diagnostic that was cancelled
    Cancelled,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_str().fmt(f)
    }
}

impl Level {
    /// Returns the ColorSpec used to style this diagnostic
    fn color(&self) -> ColorSpec {
        let mut spec = ColorSpec::new();

        match self {
            Level::Bug | Level::Error => {
                spec.set_fg(Some(ERROR_COLOR)).set_intense(true);
            }
            Level::Warning => {
                spec.set_fg(Some(WARNING_COLOR)).set_intense(true);
            }
            Level::Note => {
                spec.set_fg(Some(NOTE_COLOR)).set_intense(true);
            }
            Level::Help => {
                spec.set_fg(Some(HELP_COLOR)).set_intense(true);
            }
            Level::Cancelled => {}
        }

        spec
    }

    /// Returns the text representation of the diagnostic level
    pub fn to_str(&self) -> &'static str {
        match self {
            Level::Bug => "internal compiler error",
            Level::Error => "error",
            Level::Warning => "warning",
            Level::Note => "note",
            Level::Help => "help",
            Level::Cancelled => "cancelled",
        }
    }

    /// Returns true if this diagnostic level is considered fatal, false otherwise
    pub fn is_fatal(&self) -> bool {
        match self {
            Level::Bug | Level::Error => true,
            Level::Note | Level::Help | Level::Warning | Level::Cancelled => false,
        }
    }

    /// Converts the diagnostic level into a string using .to_str() and then styles it using the
    /// correct style for the level
    pub fn as_styled_string(&self) -> StyledString {
        let string = self.to_str();
        let style = Style::Level(*self);
        StyledString::new(String::from(string), style)
    }
}
