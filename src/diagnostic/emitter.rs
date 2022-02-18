use std::{
    io::{Error, Write},
    rc::Rc,
};

use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

use super::{
    styled::{StyledBuffer, StyledString},
    Diagnostic, Level, SourceManager, SourceName, Span,
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

        let max_spaces = self.max_line_num_width(diag);
        let mut is_first = true;

        if let Some(primary) = diag.primary {
            self.emit_line(&mut buffer, primary, None, true, max_spaces, diag.level);
            is_first = false;
        }

        for (span, label) in diag.spans.iter() {
            self.emit_line(
                &mut buffer,
                *span,
                Some(label.clone()),
                is_first,
                max_spaces,
                diag.level,
            );
            is_first = false;
        }

        for subd in diag.children.iter() {
            if let Some(span) = subd.span {
                buffer.puts(subd.level.as_styled_string());

                buffer.puts(StyledString::new(
                    format!(": {}\n", subd.message),
                    super::styled::Style::MainHeaderMsg,
                ));

                self.emit_line(&mut buffer, span, None, true, max_spaces, subd.level);
            } else {
                buffer.puts(StyledString::new(
                    format!("{:spaces$} = ", "", spaces = max_spaces),
                    super::styled::Style::LineAndColumn,
                ));

                buffer.puts(StyledString::new(
                    format!("{}: ", subd.level.to_str()),
                    super::styled::Style::MainHeaderMsg,
                ));

                buffer.puts(StyledString::new(
                    format!("{}\n", subd.message),
                    super::styled::Style::NoStyle,
                ));
            }
        }

        buffer.puts(StyledString::new(
            String::from("\n"),
            super::styled::Style::NoStyle,
        ));

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

    fn max_line_num_width(&self, diag: &Diagnostic) -> usize {
        let mut spans = Vec::new();
        let mut max_width = 0;

        if let Some(primary) = diag.primary {
            spans.push(primary);
        }

        // That could kind of be a lot of copies, is this a concern?
        for (span, _) in diag.spans.iter() {
            spans.push(*span);
        }

        for span in spans {
            let source_file = if let Some(source_file) = self.source_manager.get_file(span.source) {
                source_file
            } else {
                panic!(
                    "SourceManager recieved invalid SourceFile index from span {:?}",
                    span
                )
            };

            if let SourceName::Real(_) = &source_file.name {
                if let Some(loc) = source_file.lookup_location(span) {
                    max_width = max_width.max(format!("{}", loc.line).len());
                }
            } else {
                panic!("Unable to get the source location of a Span from a real source file");
            }
        }

        max_width
    }

    fn emit_line(
        &mut self,
        buffer: &mut StyledBuffer,
        span: Span,
        label: Option<String>,
        is_first: bool,
        max_spaces: usize,
        level: Level,
    ) {
        let source_file = if let Some(source_file) = self.source_manager.get_file(span.source) {
            source_file
        } else {
            panic!(
                "SourceManager recieved invalid SourceFile index from span {:?}",
                span
            )
        };

        if let SourceName::Real(path) = &source_file.name {
            let rel_path = pathdiff::diff_paths(path, std::env::current_dir().unwrap()).unwrap();
            let path_os = rel_path.as_os_str();
            let path = path_os.to_str().unwrap();

            if let Some(loc) = source_file.lookup_location(span) {
                let line_string = source_file.span_to_line(span).unwrap();

                let vertical_bar = StyledString::new(
                    format!("{:spaces$} | \n", "", spaces = max_spaces),
                    super::styled::Style::LineAndColumn,
                );

                if is_first {
                    //   **-->**
                    buffer.puts(StyledString::new(
                        String::from("--> "),
                        super::styled::Style::LineAndColumn,
                    ));

                    //   --> **src/file.c:2:5**
                    buffer.puts(StyledString::new(
                        format!("{}:{}:{}\n", path, loc.line + 1, loc.col + 1),
                        super::styled::Style::NoStyle,
                    ));
                }

                buffer.puts(vertical_bar.clone());

                buffer.puts(StyledString::new(
                    format!("{} | ", loc.line),
                    super::styled::Style::LineAndColumn,
                ));

                // TODO: In the future possibly cut off leading or trailing whitespace/code in such
                // a way as to not wrap to the next line even if there is a lot of code
                buffer.puts(StyledString::new(
                    format!("{}\n", line_string),
                    super::styled::Style::NoStyle,
                ));

                buffer.puts(StyledString::new(
                    format!("{:spaces$} | ", "", spaces = max_spaces),
                    super::styled::Style::LineAndColumn,
                ));

                let mut annotation = String::with_capacity(span.end - span.start);

                for _ in 0..annotation.capacity() {
                    annotation.push('^');
                }

                if let Some(label) = label {
                    buffer.puts(StyledString::new(
                        format!(
                            "{:cols$}{} {}\n",
                            "",
                            annotation,
                            label,
                            cols = (loc.col + loc.col_offset) as usize
                        ),
                        super::styled::Style::Level(level),
                    ));
                } else {
                    buffer.puts(StyledString::new(
                        format!(
                            "{:cols$}{}\n",
                            "",
                            annotation,
                            cols = (loc.col + loc.col_offset) as usize
                        ),
                        super::styled::Style::Level(level),
                    ));
                }

                buffer.puts(vertical_bar);
            } else {
                panic!("Unable to get the source location of a Span from a real source file");
            }
        } else {
            // TODO: REPLACE!
            todo!("Replace this with the code that would follow the tree of a macro expansion or anything else that isn't a real source file");
        }
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
