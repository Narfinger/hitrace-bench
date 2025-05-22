use std::{collections::HashMap, iter::Sum};

use time::Duration;

pub(crate) struct AvgMingMax<T, U> {
    pub(crate) avg: T,
    pub(crate) min: T,
    pub(crate) max: T,
    /// Please don't do more than `u16` runs.
    pub(crate) number: U,
}

pub(crate) fn avg_min_max<T, U>(durations: &[T]) -> AvgMingMax<T, U>
where
    T: Ord + Sum<T> + Copy + std::ops::Div<U, Output = T>,
    U: TryFrom<usize> + From<u16> + Copy,
{
    let number: u16 = durations.len().try_into().expect("You have too many runs");
    let min: T = *durations.iter().min().expect("Could not find min");
    let max: T = *durations.iter().max().expect("Could not find max");
    let sum: T = durations.iter().cloned().sum();
    let avg = sum / number.into();
    AvgMingMax {
        avg,
        min,
        max,
        number: number.into(),
    }
}

pub(crate) type FilterResults = HashMap<String, Vec<Duration>>;
pub(crate) type FilterErrors = HashMap<String, u32>;
pub(crate) type PointResults = HashMap<String, Vec<u64>>;

/// The results of a run given by filter.name, Vec<duration>
/// Notice that not all vectors will have the same length as some runs might fail.
pub(crate) struct RunResults {
    /// Filter results
    pub(crate) filter_results: FilterResults,
    pub(crate) errors: FilterErrors,
    pub(crate) point_results: PointResults,
}
