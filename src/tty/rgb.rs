use std::sync::LazyLock;

use super::ansi::{ColorSequence, CSI, RESET};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RgbColor(pub u8, pub u8, pub u8);

pub const RAINBOW_SIZE: usize = 20;
pub const RAINBOW_COLORS: [RgbColor; RAINBOW_SIZE] = [
    RgbColor::from_rgb(0xe71d43),
    RgbColor::from_rgb(0xff0000),
    RgbColor::from_rgb(0xff3700),
    RgbColor::from_rgb(0xff6e00),
    RgbColor::from_rgb(0xffa500),
    RgbColor::from_rgb(0xffc300),
    RgbColor::from_rgb(0xffe100),
    RgbColor::from_rgb(0xffff00),
    RgbColor::from_rgb(0xaad500),
    RgbColor::from_rgb(0x55aa00),
    RgbColor::from_rgb(0x008000),
    RgbColor::from_rgb(0x005555),
    RgbColor::from_rgb(0x002baa),
    RgbColor::from_rgb(0x0000ff),
    RgbColor::from_rgb(0x1900d5),
    RgbColor::from_rgb(0x3200ac),
    RgbColor::from_rgb(0x4b0082),
    RgbColor::from_rgb(0x812ba6),
    RgbColor::from_rgb(0xb857ca),
    RgbColor::from_rgb(0xd03a87),
];

pub const REVERSE_RAINBOW: LazyLock<[RgbColor; RAINBOW_SIZE]> = LazyLock::new(|| {
    let mut clone = RAINBOW_COLORS;
    clone.reverse();
    clone
});

impl RgbColor {
    pub const fn from_rgb(rgb: u32) -> Self {
        Self(
            (rgb >> 16) as u8,
            ((rgb >> 8) & 255) as u8,
            (rgb & 255) as u8,
        )
    }

    pub fn apply_rainbow(string: &str, background: bool, reset: bool) -> String {
        let mut buf = String::new();
        let mut reverse_order = false;

        for (index, char) in string.char_indices() {
            if (index as isize) / (RAINBOW_SIZE as isize) % 2 == 0 {
                reverse_order = true;
            } else if (index as isize) / (RAINBOW_SIZE as isize) % 2 == 1 {
                reverse_order = false;
            }

            let color = if reverse_order {
                REVERSE_RAINBOW[index % RAINBOW_SIZE]
            } else {
                RAINBOW_COLORS[index % RAINBOW_SIZE]
            };

            buf.push_str(CSI);
            buf.push_str(&color.get_sequence(background));
            buf.push_str("m");
            buf.push(char);
        }

        if reset {
            buf.push_str(RESET);
        }

        buf
    }
}

impl From<u32> for RgbColor {
    fn from(value: u32) -> Self {
        Self::from_rgb(value)
    }
}

impl From<&u32> for RgbColor {
    fn from(value: &u32) -> Self {
        value.into()
    }
}

impl ColorSequence for RgbColor {
    fn get_sequence(&self, background: bool) -> String {
        let mut sequence = if background { "48;2;" } else { "38;2;" }.to_string();

        for n in [self.0, self.1, self.2] {
            sequence += &(n.to_string() + ";");
        }

        while sequence.ends_with(';') {
            sequence.pop();
        }

        sequence
    }
}
