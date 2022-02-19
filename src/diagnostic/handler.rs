use std::{rc::Rc, sync::Mutex};

use super::{
    emitter::{Emitter, TextEmitter},
    Diagnostic, SourceManager,
};

#[derive(Debug, Copy, Clone)]
pub struct HandlerFlags {
    /// If the output should be colored or not. This should be false when the output is directed
    /// into  a file, for example
    pub colored_output: bool,
    /// Warnings can be disabled by command-line flags
    pub emit_warnings: bool,
    /// This flag determines if this Handler should actually emit any diagnostics at all. This
    /// should be set when this is being used as a library rather than an executable
    pub quiet: bool,
}

/// This is needed so that certain parts of the Handler can be behind a Mutex, so that it can be
/// mutated without Handler needing to be mutably borrowed, and so that it could theoretically be
/// safe across threads should that day come.
struct HandlerInner {
    /// The inner emitter that actually emits the Diagnostics
    pub emitter: Box<dyn Emitter>,
}

impl HandlerInner {
    pub(crate) fn new(emitter: Box<dyn Emitter>) -> Self {
        Self { emitter }
    }
}

/// A Handler handles all Diagnostics that are to be emitted through the course of compilation.
/// Diagnostics are things such as warnings and errors
pub struct Handler {
    /// The flags are provided to this Handler specifying how it should behave
    flags: HandlerFlags,
    /// The InnerHandler that actually will do the emitting of diagnostics
    inner: Mutex<HandlerInner>,
}

impl Handler {
    /// Creates a new diagnostic Handler with the provided flags and the provided emitter
    pub fn with_emitter(flags: HandlerFlags, emitter: Box<dyn Emitter>) -> Self {
        Self {
            flags,
            inner: Mutex::new(HandlerInner::new(emitter)),
        }
    }

    /// Creates a new diagnostic Handler with the provided flags and the default text emitter
    pub fn with_text_emitter(flags: HandlerFlags, source_manager: Rc<SourceManager>) -> Self {
        let emitter = Box::new(TextEmitter::new(flags.colored_output, source_manager));

        Self {
            flags,
            inner: Mutex::new(HandlerInner::new(emitter)),
        }
    }

    /// This registers a warning with this error Handler
    pub fn warn(&self, warning: Diagnostic) {
        // If we can't even emit them, don't even store them
        if self.flags.emit_warnings {
            if let Ok(mut inner) = self.inner.lock() {
                inner.emitter.emit_diagnostic(&warning);
            }
        }
    }

    /// This registers an error with this error Handler
    pub fn error(&self, error: Diagnostic) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.emitter.emit_diagnostic(&error);
        }
    }
}
