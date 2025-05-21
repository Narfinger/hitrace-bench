use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::{
    Args, Filter,
    filter::{JsonFilterDescription, PointFilter},
};

/// A RunConfig including the filters
pub(crate) struct RunConfig {
    /// The args
    pub(crate) args: Args,
    /// The filters
    pub(crate) filters: Vec<Filter>,
    /// Point filters
    pub(crate) point_filters: Vec<PointFilter>,
}

impl RunConfig {
    pub(crate) fn new(args: Args, filters: Vec<Filter>, point_filters: Vec<PointFilter>) -> Self {
        RunConfig {
            args,
            filters,
            point_filters,
        }
    }
}

/// A RunConfig which we can read from a file
/// because we need JsonFilterDescription instead of filters
#[derive(Deserialize)]
struct RunConfigDeserialize {
    args: Args,
    #[serde(default)]
    filters: Vec<JsonFilterDescription>,
    #[serde(default)]
    point_filters: Vec<PointFilter>,
}

impl From<RunConfigDeserialize> for RunConfig {
    fn from(value: RunConfigDeserialize) -> Self {
        RunConfig {
            args: value.args,
            filters: value.filters.into_iter().map(|f| f.into()).collect(),
            point_filters: value.point_filters,
        }
    }
}

/// read a run file into runs.
pub(crate) fn read_run_file(path: &PathBuf) -> Result<Vec<RunConfig>> {
    let file = File::open(path)
        .with_context(|| format!("Could not read run file {}", path.to_string_lossy()))?;
    let reader = BufReader::new(file);
    let res: Vec<RunConfigDeserialize> = serde_hjson::from_reader(reader)
        .context("Error in decoding run file. Please look at the specification")?;

    res.into_iter()
        .map(|r| {
            if r.filters.is_empty() && r.point_filters.is_empty() {
                Err(anyhow!(
                    "You did not produce a filter or pointfilter for at least one run "
                ))
            } else {
                Ok(r.into())
            }
        })
        .collect::<Result<Vec<RunConfig>>>()
}
