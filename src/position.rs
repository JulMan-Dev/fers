use std::{
    fmt::Debug,
    ops::{Bound, RangeBounds}, rc::Rc,
};

use crate::token::LexToken;


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Represents a token position, containing the file, the start and optional end.
pub struct TokenPosition {
    pub file: usize,
    pub start: usize,
    pub end: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc = r"A struct that represent a range position into a source code string."]
pub struct PositionRange(usize, Option<usize>);

pub trait ToRange {
    fn to_range(&self) -> PositionRange;
}

impl PositionRange {
    pub fn start(&self) -> usize {
        self.0
    }

    pub fn end(&self) -> Option<usize> {
        self.1
    }

    pub fn has_end(&self) -> bool {
        self.1.is_some()
    }

    pub fn size_clamped(&self, from: &Rc<str>) -> usize {
        let start = self.0.min(from.len());
        let end = self.1.unwrap_or(usize::MAX).min(from.len() - 1);

        end - start
    }

    pub fn get_slice(&self, from: &Rc<str>) -> Option<String> {
        let start = self.0.clamp(0, from.len());

        match self.1 {
            Some(end) => {
                let end = end.clamp(start, from.len());

                from.get(start..end).map(|s| s.to_owned())
            }
            None => {
                from.get(start..).map(|s| s.to_owned())
            }
        }
    }
}

impl From<LexToken> for PositionRange {
    fn from(value: LexToken) -> Self {
        Self(value.position(), Some(value.position() + value.len()))
    }
}

impl From<&LexToken> for PositionRange {
    fn from(value: &LexToken) -> Self {
        Self(value.position(), Some(value.position() + value.len()))
    }
}

impl<R> From<R> for PositionRange
where
    R: RangeBounds<usize>,
{
    fn from(value: R) -> Self {
        let mut range = PositionRange(0, None);

        match value.start_bound() {
            Bound::Included(n) => range.0 = *n,
            Bound::Excluded(n) => range.0 = n + 1,
            Bound::Unbounded => panic!("Unsupported range"),
        }

        match value.end_bound() {
            Bound::Included(n) => range.1 = Some(n + 1),
            Bound::Excluded(n) => range.1 = Some(*n),
            Bound::Unbounded => range.1 = None,
        }

        range
    }
}

impl Debug for PositionRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(end) = self.1 {
            f.write_fmt(format_args!("{}-{}", self.0, end))
        } else {
            f.write_fmt(format_args!("{}-", self.0))
        }
    }
}

impl ToRange for (usize, usize)
{
    fn to_range(&self) -> PositionRange {
        (self.0..self.1).into()
    }
}
