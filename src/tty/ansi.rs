use std::io::{self, Write};

use super::simple::Color4;

use paste::paste;

pub const CSI: &'static str = "\x1B[";
pub const RESET: &'static str = "\x1B[0m";

macro_rules! styles {
    ($vis:vis $struct:ident, [$($color:ident),+], [$($style:ident),+]) => {
        paste! {
            impl<B> $struct<Color4, B>
            where
                B: ColorSequence,
            {
                $(
                    #[doc = "Set the foreground color to " $color:lower "."]
                    $vis const fn [<with_ $color:lower _foreground>](mut self, bright: bool) -> Self {
                        self.fore = Some(Color4::$color(bright));

                        self
                    }
                )+
            }

            impl<F> $struct<F, Color4>
            where
                F: ColorSequence,
            {
                $(
                    #[doc = "Set the background color to " $color:lower "."]
                    $vis const fn [<with_ $color:lower _background>](mut self, bright: bool) -> Self {
                        self.back = Some(Color4::$color(bright));

                        self
                    }
                )+
            }

            impl<F, B> Style<F, B>
            where
                F: ColorSequence,
                B: ColorSequence
            {
                $(
                    #[doc = "Apply " $style:lower " style."]
                    $vis const fn [<with_ $style:lower>](mut self) -> Self {
                        self.$style = true;

                        self
                    }
                )+
            }
        }
    };
}

pub trait ColorSequence {
    fn get_sequence(&self, background: bool) -> String;
}

pub struct NoColor;

impl ColorSequence for NoColor {
    fn get_sequence(&self, _background: bool) -> String {
        "\x1B[D".to_owned()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Style<F, B>
where
    F: ColorSequence,
    B: ColorSequence,
{
    pub fore: Option<F>,
    pub back: Option<B>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl<F, B> Style<F, B>
where
    F: ColorSequence,
    B: ColorSequence,
{
    #[doc = "Create an empty style. With no change, it just reset after applying."]
    pub const fn empty() -> Self {
        Self {
            fore: None,
            back: None,
            bold: false,
            italic: false,
            underline: false,
        }
    }

    #[doc = "Get the apply sequence (ANSI escape sequence) for applying the style."]
    pub fn get_sequence(&self) -> String {
        let mut buf = String::new();

        if let Some(ref color) = self.fore {
            buf.push_str(CSI);
            buf.push_str(&color.get_sequence(false));
            buf.push_str("m");
        }

        if let Some(ref color) = self.back {
            buf.push_str(CSI);
            buf.push_str(&color.get_sequence(true));
            buf.push_str("m");
        }

        if self.bold {
            buf.push_str(CSI);
            buf.push_str("1m");
        }

        if self.italic {
            buf.push_str(CSI);
            buf.push_str("3m");
        }

        if self.underline {
            buf.push_str(CSI);
            buf.push_str("4m");
        }

        buf
    }

    #[doc = "Apply the style to the given string. Add style and reset sequences."]
    pub fn apply_to(&self, string: &str) -> String {
        let mut buf = self.get_sequence();

        buf.extend(string.chars());
        buf.push_str(RESET);

        buf
    }

    #[doc = "Send the style sequence to the writer (better stdout or stderr). Nexts writes will have the style."]
    pub fn send_to<T>(&self, writer: &mut T) -> io::Result<usize>
    where
        T: Write,
    {
        writer.write(self.get_sequence().as_bytes())
    }

    #[doc = "Send the reset sequence to the writer (better stdout or stderr). Nexts writes won't have the style."]
    pub fn send_reset_to<T>(&self, writer: &mut T) -> io::Result<usize>
    where
        T: Write,
    {
        writer.write(RESET.as_bytes())
    }
}

styles! {
    pub Style,
    [Black, Red, Green, Yellow, Blue, Magenta, Cyan, White],
    [bold, italic, underline]
}

#[doc = "A writer that apply style to written strings and resend them the the given output."]
pub struct StyleWriteProxy<T, F, B>
where
    T: Write,
    F: ColorSequence,
    B: ColorSequence,
{
    style: Style<F, B>,
    output: T,
    first_write: bool,
}

impl<T, F, B> StyleWriteProxy<T, F, B>
where
    T: Write,
    F: ColorSequence,
    B: ColorSequence,
{
    pub fn new(output: T, style: Style<F, B>) -> Self {
        Self {
            style,
            output,
            first_write: true,
        }
    }
}

impl<T, F, B> Write for StyleWriteProxy<T, F, B>
where
    T: Write,
    F: ColorSequence,
    B: ColorSequence,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut len = 0usize;

        if self.first_write {
            self.first_write = false;
            len += self.style.send_to(&mut self.output)?;
        }

        len += self.output.write(buf)?;

        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

impl<T, F, B> Drop for StyleWriteProxy<T, F, B>
where
    T: Write,
    F: ColorSequence,
    B: ColorSequence,
{
    fn drop(&mut self) {
        match self.style.send_reset_to(&mut self.output) {
            Ok(_) => (),
            Err(_) => panic!("Failed to send reset style to output."),
        }
    }
}
