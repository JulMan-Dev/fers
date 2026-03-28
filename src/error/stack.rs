use std::{fmt::{self, Write}, rc::Rc};
use std::collections::VecDeque;
use crate::{error::ErrorKind, position::PositionRange};

mod colors {
    use owo_colors::Style;
    pub use owo_colors::{OwoColorize};
    
    pub const RED: Style = Style::new().bright_red();
    pub const HINT: Style = Style::new().bright_blue();
    pub const CAUSED_BY: Style = Style::new().italic().underline();
}

use colors::*;

pub const CONTEXT_SIZE: usize = 3;

#[derive(Debug, Clone)]
pub struct ErrorStack {
    kind: ErrorKind,
    source: Rc<str>,
    pub range: PositionRange,
    cause: Option<Box<ErrorStack>>,
}

impl ErrorStack {
    #[doc = r"Create an error stack."]
    pub fn new(kind: ErrorKind, source: Rc<str>, range: PositionRange) -> Self {
        Self {
            kind,
            source,
            range,
            cause: None,
        }
    }

    #[doc = r"Create an error stack with a cause."]
    pub fn with_cause(
        kind: ErrorKind,
        cause: ErrorStack,
        source: Rc<str>,
        range: PositionRange,
    ) -> Self {
        // if the cause is a stack overflow, we don't want to wrap it in another stack overflow,
        // else we will produce a very large stack trace.
        // (it may use a lot of memory)
        if cause.kind == ErrorKind::StackOverflow {
            return cause;
        }
        
        Self {
            kind,
            cause: Some(Box::new(cause)),
            source,
            range,
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn range(&self) -> &PositionRange {
        &self.range
    }

    pub fn cause(&self) -> Option<&Box<ErrorStack>> {
        self.cause.as_ref()
    }

    pub fn format_line(&self, colorize: bool) -> Result<String, fmt::Error> {
        let mut f = String::new();

        if let Some(str) = self.range.get_slice(&self.source) {
            let error_range = if self.range.has_end() {
                str.len()
            } else {
                str.trim_end().len()
            };

            f.write_str("\n")?;
            f.write_fmt(format_args!("  {}\n", &self.source))?;
            write!(
                f,
                "  {}",
                (" ".repeat(self.range.start()) + &"━".repeat(error_range))
                .style(RED)
            )?;

            if self.kind == ErrorKind::UnclosedStringLiteral {
                write!(
                    f,
                    "{}",
                    format!(
                    "┃\n  {}┗ Consider closing the opened quote here",
                    " ".repeat(self.range.start() + error_range)
                    ).style(HINT)
                )?;
            }

            if let ErrorKind::InvalidNumberArguments(excepted, found) = self.kind {
                write!(
                    f,
                    "{}",
                    format!("\n  {}┗ Consider adding {} argument{} before this call",
                    " ".repeat(self.range.start()),
                    excepted - found,
                    if (0..=1).contains(&(excepted - found)) {
                        ""
                    } else {
                        "s"
                    }).style(HINT)
                )?;
            }

            if self.kind == ErrorKind::UnexpectedToken
                && self.range.start() >= 1
                && self.source.chars().nth(self.range.start() - 1) == Some('"')
            {
                write!(
                    f,
                    "{}",
                    format!(
                    "\n  {}┗ You must insert a whitespace just after string literal",
                    " ".repeat(self.range.start()),
                    ).style(HINT)
                )?;
            }

            f.write_str("\n\n")?;
        }

        write!(f, "{}", self.kind())?;

        // cause should be formatted using line
        if let Some(ref cause) = self.cause {
            write!(f, "{}", "\n\nCaused by:".to_string().style(CAUSED_BY))?;

            let formatted = cause.format_line(colorize)?;
            let lines: Vec<&str> = formatted.lines().collect();
            let mut buf = String::new();

            for line in lines {
                buf += &("   ".to_string() + line + "\n");
            }

            write!(f, "{}", buf)?;
        }

        Ok(f)
    }

    pub fn format_chunk(&self) -> Result<String, fmt::Error> {
        let f = String::new();
    
        let lines: Vec<_> = self.source.lines().collect();
        let lines = {
            let start_i = self.range.start();
            let end_i = self.range.end().unwrap_or(self.source.len() - 1);
    
            let mut i = 0usize;
            let mut line_i = 0usize;
            let mut context = VecDeque::new();
    
            for line in lines {
                line_i += 1;
                i += line.len();
    
                context.push_back(line);
    
                if i + 1 <= start_i + line.len() && start_i < i {
                    context = context.split_off(line_i - 1 - CONTEXT_SIZE);
                }
            }
    
            context
        };
    
        Ok(f)
    }
}
