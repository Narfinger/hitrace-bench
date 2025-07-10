use std::sync::LazyLock;

use itertools::Itertools;
use log::error;
use regex::{Captures, Regex};
use serde::Deserialize;

use crate::{
    runconfig::RunConfig,
    trace::{Trace, TraceMarker},
};

const SERVO_MEMORY_PROFILING_STRING: &str = "servo_memory_profiling";

/// We have different type of points which have different regexp.
/// See the statics for a detailed explanation
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PointType {
    /// A memory report that has an url attached, like LayoutThread.
    MemoryUrl,
    /// A simple memory report, corresponding to resident-memory most likely.
    MemoryReport,
    /// Report of smaps.
    Smaps,
    /// Report of a testcase point.
    Testcase,
    /// Something we will combine.
    Combined,
}

/// Notice that this also matches MEMORY_URL_REPORT
/// This is the general regexp
/// Example: servo_memory_profiling:resident 270778368
static MEMORY_REPORT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^servo_memory_profiling:(.*?)\s(\d+)$").expect("Could not parse regexp")
});

/// Reports that contain an url/iframe
/// Example: servo_memory_profiling:url(https://servo.org/)/js/non-heap 262144
static MEMORY_URL_REPORT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^servo_memory_profiling:url\((.*?)\)\/(.*?)\s(\d+)$")
        .expect("Could not parse regexp")
});

/// resident-according-to-smaps has again a different way
/// Example: servo_memory_profiling:resident-according-to-smaps/other 60424192
static SMAPS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^servo_memory_profiling:(resident-according-to-smaps)\/(.*)\s(\d+)$")
        .expect("Could not parse regexp")
});

/// Example: TESTCASE_PROFILING: generatehtml:data 2453
static TESTCASE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^TESTCASE_PROFILING: ([A-z]+):?([A-z]*) (\d+)$").expect("Could not parse regexp")
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PointValue {
    /// Size in kb
    Size(u64),
    /// Other measurement
    Custom(String, u64),
}

impl PointValue {
    pub(crate) fn value(&self) -> u64 {
        match self {
            PointValue::Size(v) => *v,
            PointValue::Custom(_, v) => *v,
        }
    }
}

#[derive(Debug)]
/// A parsed trace point metric
pub(crate) struct Point<'a> {
    /// The name you gave to this point
    pub(crate) name: String,
    /// The value of the point
    pub(crate) point_value: PointValue,
    /// The type of point this matches to
    pub(crate) point_type: PointType,
    /// The trace this matches to
    pub(crate) trace: Option<&'a Trace>,
}

/// You might want to extract data points. These do not have a beginning and end, just a point.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PointFilter {
    /// The name we will use for this string
    pub(crate) name: String,
    /// We substring match on this
    pub(crate) match_str: String,
    /// With this we combine all points that match a substring
    #[serde(default)]
    pub(crate) combined: bool,
}

impl PointFilter {
    pub(crate) fn new(name: String, match_str: String) -> Self {
        PointFilter {
            name,
            match_str,
            combined: false,
        }
    }

    /// This filters sub memory reports with a url attached.
    fn filter_memory_url<'a>(
        &'a self,
        run_config: &RunConfig,
        groups: Captures,
        trace: &'a Trace,
    ) -> Option<Point<'a>> {
        let url = groups.get(1).expect("No match").as_str();
        let fn_name = groups.get(2).expect("No match").as_str();
        let value = groups
            .get(3)
            .expect("No match")
            .as_str()
            .parse()
            .expect("Could not parse value");
        if url.contains(run_config.run_args.url.as_str()) {
            let mut suffix = fn_name.split('/').skip(1).join("/");
            if !suffix.is_empty() {
                suffix.insert(0, '/');
            }
            Some(Point {
                name: run_config.run_args.url.to_owned()
                    + "/"
                    + self.name.as_str()
                    + suffix.as_str(),
                point_value: PointValue::Size(value),
                trace: Some(trace),
                point_type: PointType::MemoryUrl,
            })
        } else {
            None
        }
    }

    /// This filters smaps reports
    fn filter_smaps<'a>(
        &'a self,
        run_config: &RunConfig,
        groups: Captures,
        trace: &'a Trace,
    ) -> Option<Point<'a>> {
        if groups.get(1).unwrap().as_str() != self.match_str {
            None
        } else {
            let value = groups
                .get(3)
                .expect("Could not find match")
                .as_str()
                .parse()
                .expect("Could not parse");
            Some(Point {
                name: run_config.run_args.url.to_owned() + "/" + self.name.as_str(),
                point_value: PointValue::Size(value),
                trace: Some(trace),
                point_type: PointType::Smaps,
            })
        }
    }

    /// This simple memory reports
    fn filter_memory<'a>(
        &'a self,
        run_config: &RunConfig,
        groups: Captures,
        trace: &'a Trace,
    ) -> Option<Point<'a>> {
        let value = groups
            .get(2)
            .expect("Could not find match")
            .as_str()
            .parse()
            .expect("Could not parse value");
        Some(Point {
            name: run_config.run_args.url.to_owned() + "/" + self.name.as_str(),
            point_value: PointValue::Size(value),
            trace: Some(trace),
            point_type: PointType::MemoryReport,
        })
    }

    /// This filters test cases
    fn filter_testcase<'a>(
        &'a self,
        run_config: &RunConfig,
        groups: Captures,
        trace: &'a Trace,
    ) -> Option<Point<'a>> {
        let case_name = groups.get(1).expect("Could not find match").as_str();
        let data_name = groups
            .get(2)
            .expect("Could not find match")
            .as_str()
            .to_owned();
        let value = groups
            .get(3)
            .expect("Could not find match")
            .as_str()
            .parse()
            .expect("Could not parse value");
        if case_name.contains(&self.match_str) {
            Some(Point {
                name: run_config.run_args.url.to_owned() + "/",
                point_value: PointValue::Custom(data_name, value),
                trace: Some(trace),
                point_type: PointType::Testcase,
            })
        } else {
            None
        }
    }

    /// This is the main filter function which will call the corresponding filter functions
    fn filter_trace_to_option_point<'a>(
        &'a self,
        trace: &'a Trace,
        run_config: &RunConfig,
    ) -> Option<Point<'a>> {
        if let Some(groups) = MEMORY_URL_REPORT_REGEX.captures(&trace.function) {
            self.filter_memory_url(run_config, groups, trace)
        } else if let Some(groups) = SMAPS_REGEX.captures(&trace.function) {
            self.filter_smaps(run_config, groups, trace)
        } else if let Some(groups) = MEMORY_REPORT_REGEX.captures(&trace.function) {
            self.filter_memory(run_config, groups, trace)
        } else if let Some(groups) = TESTCASE_REGEX.captures(&trace.function) {
            self.filter_testcase(run_config, groups, trace)
        } else {
            None
        }
    }

    /// Check if there are duplicates for PointType::Testcase and PointType::MemoryReport.
    /// Remove these and print errors.
    fn remove_duplicates(&self, points: &mut Vec<Point>) {
        if points
            .iter()
            .filter(|p| p.point_type == PointType::Testcase)
            .count()
            > 1
        {
            error!(
                "PointFilter {:?} matched with multiple traces {:?}. Discarding",
                self,
                points
                    .iter()
                    .filter_map(|p| p.trace)
                    .collect::<Vec<&Trace>>()
            );
            points.retain(|p| p.point_type != PointType::Testcase);
        }

        if points
            .iter()
            .filter(|p| p.point_type == PointType::MemoryReport)
            .count()
            > 1
        {
            error!(
                "PointFilter {:?} matched with multiple traces {:?}. Discarding",
                self,
                points
                    .iter()
                    .filter_map(|p| p.trace)
                    .collect::<Vec<&Trace>>()
            );
            points.retain(|p| p.point_type != PointType::MemoryReport);
        }
    }

    /// Takes a a `PointFilter`, an array of traces and a run_config to create a result of matched points.
    pub(crate) fn pointfilter_to_point<'a>(
        &'a self,
        traces: &'a [Trace],
        run_config: &'a RunConfig,
    ) -> Vec<Point<'a>> {
        let mut points: Vec<_> = traces
            .iter()
            .filter(|t| t.trace_marker == TraceMarker::Dot)
            .filter(|t| {
                t.function.contains(SERVO_MEMORY_PROFILING_STRING)
                    || t.function.contains("TESTCASE_PROFILING")
            })
            .filter(|t| t.function.contains(&self.match_str))
            .filter_map(|t| self.filter_trace_to_option_point(t, run_config))
            .collect();

        if self.combined {
            // we now need to collect points with the same name
            points
                .into_iter()
                .into_group_map_by(|p| p.name.clone())
                .into_iter()
                .map(|(name, mut vals)| {
                    if vals.len() == 1 {
                        vals.remove(0)
                    } else {
                        Point {
                            name,
                            point_value: PointValue::Size(vals.iter().map(|p| match p.point_value{
                                PointValue::Size(s) => s,
                                PointValue::Custom(_, _) => {
                                    error!("Combined Custom Points are not supported at the moment"); 0
                                },
                            }).sum()),
                            trace: None,
                            point_type: PointType::Combined,
                        }
                    }
                })
                .collect()
        } else {
            self.remove_duplicates(&mut points);
            points
        }
    }
}
