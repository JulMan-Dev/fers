use std::time::Duration;

use rust_decimal::Decimal;

use crate::error::ErrorKind;

pub const MAX_DECIMAL: i128 = 79_228_162_514_264_337_593_543_950_335;
pub const MIN_DECIMAL: i128 = -79_228_162_514_264_337_593_543_950_335;

pub fn integer_to_float(i: i128) -> Result<Decimal, ErrorKind> {
    if i > MAX_DECIMAL || i < MIN_DECIMAL {
        Err(ErrorKind::SizeOverflow)
    } else {
        Ok(Decimal::from_i128_with_scale(i, 0))
    }
}

pub trait DisplayMinimized {
    fn display_minimized(&self) -> String;
}

impl DisplayMinimized for Duration {
    fn display_minimized(&self) -> String {
        if self.as_nanos() < 1000 {
            self.as_nanos().to_string() + "ns"
        } else if self.as_micros() < 1000 {
            self.as_micros().to_string() + "µs"
        } else if self.as_millis() < 1000 {
            self.as_millis().to_string() + "ms"
        } else {
            format!("{:.2}s", self.as_secs_f32())
        }
    }
}

pub trait VecUtils<T> {
    fn remove_last(&mut self);
    #[doc = r"Remove element with index starting by the end."]
    fn remove_reversed(&mut self, index: usize);
    fn get_last(&self) -> &T;
    fn take_last_chunk<const N: usize>(&mut self) -> Option<[T; N]>;
}

impl<T> VecUtils<T> for Vec<T> {
    fn remove_last(&mut self) {
        self.remove_reversed(1);
    }

    fn remove_reversed(&mut self, index: usize) {
        self.remove(self.len() - index);
    }

    fn get_last(&self) -> &T {
        &self[self.len() - 1]
    }

    fn take_last_chunk<const N: usize>(&mut self) -> Option<[T; N]> {
        if N > self.len() {
            None
        } else {
            let chunk = self.split_off(self.len() - N);
            chunk.try_into().ok()
        }
    }
}
