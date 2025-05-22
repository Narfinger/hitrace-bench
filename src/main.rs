use anyhow::{Context, Result, anyhow};
use args::Args;
use clap::Parser;
use filter::{Filter, PointFilter};
use runconfig::RunConfig;
use std::collections::HashMap;
use time::Duration;
use trace::{Point, Trace};
use yansi::{Condition, Paint};

mod args;
mod bencher;
mod device;
mod filter;
mod runconfig;
mod trace;

struct AvgMingMax {
    avg: Duration,
    min: Duration,
    max: Duration,
    number: usize,
}

fn avg_min_max(durations: &[Duration]) -> Option<AvgMingMax> {
    let number = durations.len();
    durations
        .iter()
        .min()
        .zip(durations.iter().max())
        .map(|(min, max)| AvgMingMax {
            avg: durations.iter().sum::<Duration>() / number as f64,
            min: *min,
            max: *max,
            number,
        })
}

/// Print the differences
fn print_differences(
    args: &Args,
    results: &RunResults,
    errors: &HashMap<String, u32>,
    points: &[Vec<Point>],
) {
    println!("The following things broke with errors");
    for (key, val) in errors.iter() {
        println!("{}: {} errors", key, val);
    }

    println!(
        "----name {} {} {}------({}) runs (hp:{})------------------------",
        "avg".yellow(),
        "min".green(),
        "max".red(),
        args.tries,
        args.url
    );
    for (key, val) in results.iter() {
        if let Some(avg_min_max) = avg_min_max(val) {
            println!(
                "{}: {} {} {}  ({} runs)",
                key,
                avg_min_max.avg.yellow().whenever(Condition::TTY_AND_COLOR),
                avg_min_max.min.green().whenever(Condition::TTY_AND_COLOR),
                avg_min_max.max.red().whenever(Condition::TTY_AND_COLOR),
                avg_min_max.number,
            );
        } else {
            println!("{}: _ _ _  (0 runs)", key);
        }
    }

    if !points.is_empty() {
        println!("-----------Points-------------------------");
        for i in points.iter().flatten() {
            println!("{}: {}", i.name, i.value);
        }
    }
}

/// The results of a run given by filter.name, Vec<duration>
/// Notice that not all vectors will have the same length as some runs might fail.
type RunResults = HashMap<String, Vec<Duration>>;

/// Runs one RunConfig and append the results to the results, errors and points
fn run_runconfig(
    run_config: &RunConfig,
    use_bencher: bool,
    results: &mut HashMap<String, Vec<Duration>>,
    errors: &mut HashMap<String, u32>,
    points: &mut Vec<Vec<Point>>,
) -> Result<()> {
    for i in 1..run_config.args.tries + 1 {
        if !run_config.args.bencher {
            println!("Running test {}", i);
        }
        let traces = if let Some(ref file) = run_config.args.trace_file {
            device::read_file(file)?
        } else {
            let log_path = device::exec_hdc_commands(&run_config.args)?;
            device::read_file(&log_path)?
        };

        // Collect differences
        let differences = filter::find_notable_differences(&traces, &run_config.filters);
        for (original_key, value) in differences.into_iter() {
            let key = if use_bencher {
                let new_key = format!("E2E/{}/{}", run_config.args.url, original_key);
                new_key
            } else {
                original_key.to_owned()
            };
            if let Ok(d) = value {
                results
                    .entry(key)
                    .and_modify(|v| v.push(d))
                    .or_insert(vec![(d)]);
            } else {
                errors.entry(key).and_modify(|v| *v += 1).or_insert(1);
            }

            let new_points: Vec<Point> = run_config
                .point_filters
                .iter()
                .flat_map(|f| f.pointfilter_to_point(&traces))
                .collect();
            points.push(new_points);
        }

        if run_config.args.tries == 1 && run_config.args.all_traces {
            println!("Printing {} traces", &traces.len());
            for i in &traces {
                println!("{:?}", i);
            }
            println!("----------------------------------------------------------\n\n");
        }
    }
    Ok(())
}

/// Runs runconfigs
/// Bencher has to be treated separately because it wants a valid json output.
fn run_runconfigs(run_configs: &Vec<RunConfig>, use_bencher: bool) -> Result<()> {
    let mut results: HashMap<String, Vec<Duration>> = HashMap::new();
    let mut errors: HashMap<String, u32> = HashMap::new();
    let mut points = Vec::new();
    for run_config in run_configs {
        run_runconfig(
            run_config,
            use_bencher,
            &mut results,
            &mut errors,
            &mut points,
        )?;

        if !use_bencher {
            print_differences(&run_config.args, &results, &errors, &points);
            results = HashMap::new();
            errors = HashMap::new();
        }
    }

    if use_bencher {
        bencher::write_results(results, points)
    }
    Ok(())
}

fn main() -> Result<()> {
    let run_configs: Vec<RunConfig> = {
        let args = Args::parse();
        if let Some(file) = args.run_file {
            runconfig::read_run_file(&file)?
        } else if let Some(ref path) = args.filter_file {
            let filters = filter::read_filter_file(path)?;
            vec![RunConfig::new(args, filters, vec![])]
        } else {
            let filters = vec![
                Filter {
                    name: String::from("Surface->LoadStart"),
                    first: Box::new(|t: &Trace| t.function.contains("on_surface_created_cb")),
                    last: Box::new(|t: &Trace| t.function.contains("load status changed Head")),
                },
                Filter {
                    name: String::from("Load->Compl"),
                    first: Box::new(|t: &Trace| t.function.contains("load status changed Head")),
                    last: Box::new(|t: &Trace| t.function.contains("PageLoadEndedPrompt")),
                },
            ];
            let point_filters = vec![PointFilter {
                name: String::from("VSize"),
                match_str: String::from("servo_memory_profiling:"),
            }];
            vec![RunConfig::new(args, filters, point_filters)]
        }
    };

    if !device::is_device_reachable().context("Testing reachability of device")? {
        return Err(anyhow!("No phone seems to be reachable"));
    }

    let trace_buffer = run_configs
        .first()
        .expect("Need at least one RunConfig")
        .args
        .trace_buffer;

    let all_bencher = run_configs.iter().all(|r| r.args.bencher);
    let all_print = run_configs.iter().all(|r| !r.args.bencher);
    if !all_bencher && !all_print {
        println!("We only support all bencher or all print runs");
        return Ok(());
    }

    ctrlc::set_handler(move || {
        device::stop_tracing(trace_buffer).expect("Could not stop tracing");
    })?;

    run_runconfigs(&run_configs, all_bencher)?;

    Ok(())
}
