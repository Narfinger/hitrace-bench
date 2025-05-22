use std::{collections::HashMap, fs::File, io::BufWriter};

use rust_decimal::Decimal;
use serde::Serialize;
use time::Duration;

use crate::{Point, RunResults, avg_min_max};

#[derive(Debug, Serialize)]
/// Struct for bencher json
struct LoadSpeed {
    #[serde(with = "rust_decimal::serde::float")]
    value: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    lower_value: Decimal,
    #[serde(with = "rust_decimal::serde::float")]
    upper_value: Decimal,
}

#[derive(Debug, Serialize)]
struct SingleBencherPoint {
    value: Decimal,
}

/// Converts duration to bencher Decimal representation. Duration has precision of nanoseconds
fn difference_to_bencher_decimal(dur: &Duration) -> Decimal {
    let number = dur.whole_nanoseconds();
    Decimal::from_i128_with_scale(number, 0)
}

type BencherLatency<'a> = HashMap<&'a str, LoadSpeed>;
type BencherPoint<'a> = HashMap<&'a str, SingleBencherPoint>;
#[derive(Serialize)]
#[serde(untagged)]
enum Bencher<'a> {
    LoadSpeed(BencherLatency<'a>),
    Point(BencherPoint<'a>),
}

/// Output in bencher json format to bench.json
/// We also will append it to the bench.json file instead of overwriting it so supsequent runs can be recorded.
/// We also add some custom strings to the filter.
pub(crate) fn write_results(result: RunResults, points: Vec<Vec<Point>>) {
    let filters_iter = result.into_iter().map(|(key, dur_vec)| {
        let avg_min_max = avg_min_max(&dur_vec);
        // yes we need this hashmap for the correct json
        let mut map = HashMap::new();
        if let Some(avg_min_max) = avg_min_max {
            map.insert(
                "LoadSpeed",
                LoadSpeed {
                    value: difference_to_bencher_decimal(&avg_min_max.avg),
                    lower_value: difference_to_bencher_decimal(&avg_min_max.min),
                    upper_value: difference_to_bencher_decimal(&avg_min_max.max),
                },
            );
        }
        (key, Bencher::LoadSpeed(map))
    });

    let points_iter = points.into_iter().flatten().map(|p| {
        let mut map = HashMap::new();
        map.insert(
            "Memory",
            SingleBencherPoint {
                value: Decimal::from_i128_with_scale(p.value as i128, 0),
            },
        );
        (p.name, Bencher::Point(map))
    });

    let b: HashMap<String, Bencher> = filters_iter.chain(points_iter).collect();

    let file = File::create("bench.json").expect("Could not open file");
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &b).expect("Could not write json");
    println!(
        "{}",
        serde_json::to_string_pretty(&b).expect("Could not serialize")
    );
}
