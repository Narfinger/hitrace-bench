use std::{collections::HashMap, iter::Sum};

use time::Duration;

use crate::Point;

pub(crate) struct AvgMingMax<T> {
    pub(crate) avg: T,
    pub(crate) min: T,
    pub(crate) max: T,
    pub(crate) number: usize,
}

pub(crate) fn avg_min_max<T>(durations: &[T]) -> AvgMingMax<T>
where
    T: Ord + Sum<T> + Copy,
    T: std::ops::Div<u32, Output = T>,
{
    let number = durations.iter().count();
    let min: T = *durations.iter().min().expect("Could not find min");
    let max: T = *durations.iter().max().expect("Could not find max");
    let sum: T = durations.iter().cloned().sum();
    let avg = sum / (number as u32);
    AvgMingMax {
        avg,
        min,
        max,
        number,
    }
}

/// The results of a run given by filter.name, Vec<duration>
/// Notice that not all vectors will have the same length as some runs might fail.
pub(crate) struct RunResults {
    /// Filter results
    pub(crate) filter_results: HashMap<String, Vec<Duration>>,
    pub(crate) errors: HashMap<String, Vec<Duration>>,
    pub(crate) point_results: HashMap<String, Vec<Point>>,
}
