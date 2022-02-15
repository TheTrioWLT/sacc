use std::{
    io::{Error, Write},
    rc::Rc,
};

use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

use super::{
    styled::{StyledBuffer, StyledString},
    Diagnostic, SourceFile, SourceManager, SourceName,
};

/// A trait describing a type that can emit diagnostics
pub trait Emitter {
    /// Emit a diagnostic
    fn emit_diagnostic(&mut self, diag: &Diagnostic);
}

/// A type that implements Emitter that is to be used for standard text diagnostics such as in
/// standard I/O or files
///
/// This would be in contrast to a hypothetical JSON Emitter that would allow for easier language
/// server integration
///
pub struct TextEmitter {
    colored: bool,
    source_manager: Rc<SourceManager>,
}

impl Emitter for TextEmitter {
    fn emit_diagnostic(&mut self, diag: &Diagnostic) {
        if let Err(e) = self.emit_diagnostic_inner(diag) {
            panic!("Error emitting diagnostic: {}", e);
        }
    }
}

impl TextEmitter {
    /// Creates a new TextEmitter using the provided options and SourceManager
    pub fn new(colored: bool, source_manager: Rc<SourceManager>) -> Self {
        Self {
            colored,
            source_manager,
        }
    }

    /// Gets a handle to stderr to be used for emitting diagnostics
    fn get_stderr(&self) -> StandardStream {
        let choice = if self.colored {
            // I've found that Auto works the best for colored output
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };

        StandardStream::stderr(choice)
    }

    /// This version of the function is capable of returning a Result which must be handled by the
    /// implementation that calls it
    fn emit_diagnostic_inner(&mut self, diag: &Diagnostic) -> Result<(), Error> {
        let mut stream = self.get_stderr();
        let mut buffer = StyledBuffer::new();

        // **level:**
        buffer.puts(diag.level.as_styled_string());

        // level: **message**
        buffer.puts(StyledString::new(
            format!(": {}\n", &diag.message),
            super::styled::Style::MainHeaderMsg,
        ));

        if let Some(primary) = diag.primary {
            let source_file =
                if let Some(source_file) = self.source_manager.get_file(primary.source) {
                    source_file
                } else {
                    panic!(
                        "SourceManager recieved invalid SourceFile index from span {:?}",
                        primary
                    )
                };

            if let SourceName::Real(path) = &source_file.name {
                let rel_path =
                    pathdiff::diff_paths(path, std::env::current_dir().unwrap()).unwrap();
                let path_os = rel_path.as_os_str();
                let path = path_os.to_str().unwrap();

                if let Some((line, col)) = source_file.lookup_location(primary) {
                    let line_string = source_file.span_to_line(primary).unwrap();

                    let line_num_size = format!("{}", line).len();

                    let vertical_bar = StyledString::new(
                        format!("{:spaces$} | \n", "", spaces = line_num_size),
                        super::styled::Style::LineAndColumn,
                    );

                    //   **-->**
                    buffer.puts(StyledString::new(
                        format!("--> "),
                        super::styled::Style::LineAndColumn,
                    ));

                    //   --> **src/file.c:2:5**
                    buffer.puts(StyledString::new(
                        format!("{}:{}:{}\n", path, line + 1, col + 1),
                        super::styled::Style::NoStyle,
                    ));

                    buffer.puts(vertical_bar.clone());

                    buffer.puts(StyledString::new(
                        format!("{} | ", line),
                        super::styled::Style::LineAndColumn,
                    ));

                    // TODO: In the future possibly cut off leading or trailing whitespace/code in such
                    // a way as to not wrap to the next line even if there is a lot of code
                    buffer.puts(StyledString::new(
                        format!("{}\n", line_string),
                        super::styled::Style::NoStyle,
                    ));

                    buffer.puts(vertical_bar);
                } else {
                    panic!("Unable to get the source location of a Span from a real source file");
                }
            } else {
                // TODO: REPLACE!
                todo!("Replace this with the code that would follow the tree of a macro expansion or anything else that isn't a real source file");
            }
        }

        // Render the buffer we have accumulated
        self.render_buffer(&mut stream, &buffer)?;

        Ok(())
    }

    fn render_buffer(
        &self,
        stream: &mut StandardStream,
        buffer: &StyledBuffer,
    ) -> Result<(), Error> {
        for string in buffer.iter() {
            let spec = string.style.to_spec();

            if self.colored {
                stream.set_color(&spec)?;
            }

            write!(stream, "{}", string.text)?;

            if self.colored {
                stream.set_color(&ColorSpec::new())?;
            }
        }

        Ok(())
    }

    fn emit_line(&mut self, buffer: &mut StyledBuffer, file: Rc<SourceFile>, line: &Line) {
        todo!();
    }
}

#[derive(Debug)]
pub struct Line {
    /// The line number of this source line, 1-indexed
    pub line: usize,

    /// Annotations as part of diagnostic messages
    pub annotations: Vec<Annotation>,
}

#[derive(Debug)]
pub struct Annotation {
    /// Start column, 0-based indexing, counting *characters* not, UTF-8 bytes
    pub start_col: usize,

    /// End column within the line
    pub end_col: usize,

    /// Optional label to display next to the annotation
    pub label: Option<String>,
}
