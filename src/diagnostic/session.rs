use std::{path::Path, rc::Rc};

use super::{DiagnosticBuilder, Handler, SourceFile, SourceManager, Span};

pub struct Session {
    // TODO: Add command line configuration into here
    source_manager: Rc<SourceManager>,
    handler: Handler,
}

impl Session {
    pub fn new(source_manager: Rc<SourceManager>, handler: Handler) -> Self {
        Self {
            source_manager,
            handler,
        }
    }

    pub fn load_file(&self, path: &Path) -> std::io::Result<Rc<SourceFile>> {
        self.source_manager.load_file(path)
    }

    pub fn source_manager(&self) -> &SourceManager {
        self.source_manager.as_ref()
    }

    pub fn struct_span_error(&self, span: Span, message: impl Into<String>) -> DiagnosticBuilder {
        let mut db = DiagnosticBuilder::new(&self.handler, super::Level::Error, message.into());

        db.set_primary_span(span);

        db
    }

    pub fn struct_span_warn(&self, span: Span, message: impl Into<String>) -> DiagnosticBuilder {
        let mut db = DiagnosticBuilder::new(&self.handler, super::Level::Warning, message.into());

        db.set_primary_span(span);

        db
    }

    pub fn struct_bug(&self, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(&self.handler, super::Level::Bug, message.into())
    }

    pub fn struct_error(&self, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(&self.handler, super::Level::Error, message.into())
    }

    pub fn struct_warn(&self, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(&self.handler, super::Level::Warning, message.into())
    }

    /// Returns the String that is contained in the span provided
    pub fn span_to_string(&self, span: Span) -> Option<String> {
        self.source_manager.span_to_string(span)
    }
}
