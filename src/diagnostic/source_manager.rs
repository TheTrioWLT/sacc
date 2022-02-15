use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use elsa::FrozenVec;

use super::{Loc, Span};

/// A name for a source of tokens
#[derive(Debug)]
pub enum SourceName {
    /// A real file that was included for compilation
    Real(PathBuf),
    /// Tokens created from the result of a macro expansion
    MacroExpansion(Span),
}

/// The item in which Spans are an index into, whether an actual source file or a macro expansion
#[derive(Debug)]
pub struct SourceFile {
    pub name: SourceName,
    /// The index into the SourceManager's Vec<SourceFile> that this SourceFile is positioned at
    pub index: usize,
    pub src: Option<String>,
    pub lines: Vec<(usize, usize)>,
}

impl SourceFile {
    pub fn new(name: SourceName, src: Option<String>, index: usize) -> Self {
        let mut lines = Vec::new();

        if let Some(ref src) = src {
            let mut index = 0;

            for line in src.split('\n') {
                lines.push((index, index + line.len()));
                index += line.len();
            }
        }

        Self {
            name,
            index,
            src,
            lines,
        }
    }

    /// Gets the index into this SourceFile's lines Vec that this span is in
    fn get_line(&self, span: Span) -> Option<usize> {
        for (line, (begin, end)) in self.lines.iter().enumerate() {
            if span.start >= *begin && span.start < *end {
                return Some(line);
            }
        }

        None
    }

    pub fn span_to_string(&self, span: Span) -> Option<String> {
        let src = self.src.as_ref()?;

        Some((&src[span.start..span.end]).to_string())
    }

    pub fn span_to_line(&self, span: Span) -> Option<String> {
        let index = self.get_line(span)?;
        let line = self.lines.get(index)?;

        let src = self.src.as_ref()?;

        Some((&src[line.0..line.1]).to_string())
    }

    /// Returns the source file (0-indexed line number, 0-indexed column) for the given Span, based
    /// off of the span.start
    pub fn lookup_location(&self, span: Span) -> Option<(usize, usize)> {
        let index = self.get_line(span)?;
        let line = self.lines.get(index)?;

        Some((index, span.start - line.0))
    }
}

pub struct SourceManager {
    files: FrozenVec<Box<Rc<SourceFile>>>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            files: FrozenVec::new(),
        }
    }

    /// Checks for the existence of a file
    pub fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    pub fn add_file(&self, mut source_file: SourceFile) -> Rc<SourceFile> {
        // Assign the correct index
        source_file.index = self.files.len();

        let source_file = Rc::new(source_file);

        self.files.push(Box::new(source_file.clone()));

        source_file
    }

    pub fn load_file(&self, path: &Path) -> std::io::Result<Rc<SourceFile>> {
        let src = std::fs::read_to_string(path)?;
        let filename = SourceName::Real(std::fs::canonicalize(path)?);

        self.new_source_file(filename, src)
    }

    fn new_source_file(
        &self,
        filename: SourceName,
        src: String,
    ) -> std::io::Result<Rc<SourceFile>> {
        let source_file = SourceFile::new(filename, Some(src), self.files.len());

        Ok(self.files.push_get(Box::new(Rc::new(source_file))).clone())
    }

    /// Returns a reference to a SourceFile at the given index
    pub fn get_file(&self, index: usize) -> Option<Rc<SourceFile>> {
        self.files.get(index).cloned()
    }

    /// Returns a Loc that represents where this span is inside of a source file
    pub fn lookup_location(&self, span: Span) -> Option<Loc> {
        let source_file = self.files.get(span.source)?;

        let (line, col) = source_file.lookup_location(span)?;

        Some(Loc::new(source_file.clone(), line, col))
    }

    /// Returns the String that represents the entire source line that a Span begins on
    pub fn span_to_line(&self, span: Span) -> Option<String> {
        let source_file = self.files.get(span.source)?;

        source_file.span_to_line(span)
    }

    /// Returns the String that is contained in the span provided
    pub fn span_to_string(&self, span: Span) -> Option<String> {
        let source_file = self.files.get(span.source)?;

        source_file.span_to_string(span)
    }
}
