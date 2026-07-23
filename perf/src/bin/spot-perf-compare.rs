use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = run() {
        eprintln!("[spot][perf-compare] {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let baseline = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: spot-perf-compare BASELINE_DIR CURRENT_DIR [--tolerance PERCENT]")?;
    let current = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: spot-perf-compare BASELINE_DIR CURRENT_DIR [--tolerance PERCENT]")?;
    let mut tolerance = 10.0f64;
    while let Some(arg) = args.next() {
        if arg == "--tolerance" {
            tolerance = args
                .next()
                .ok_or("--tolerance requires a percentage")?
                .parse()
                .map_err(|_| "invalid tolerance percentage")?;
        } else {
            return Err(format!("unknown argument: {arg}"));
        }
    }
    if tolerance < 0.0 {
        return Err("tolerance cannot be negative".to_string());
    }

    let mut regressions = 0usize;
    let summary_files = summary_files(&baseline)?;
    if summary_files.is_empty() {
        return Err(format!(
            "no *.summary.csv files found in {}",
            baseline.display()
        ));
    }
    for baseline_file in summary_files {
        let name = baseline_file
            .file_name()
            .ok_or("invalid baseline filename")?;
        let current_file = current.join(name);
        if !current_file.exists() {
            eprintln!("[spot][perf-compare] MISSING {}", current_file.display());
            regressions += 1;
            continue;
        }
        let base = read_single_row_csv(&baseline_file)?;
        let candidate = read_single_row_csv(&current_file)?;
        let scenario = candidate
            .get("scenario")
            .map(String::as_str)
            .unwrap_or_else(|| name.to_str().unwrap_or("unknown"));
        for metric in [
            "p95_frame_interval_ms",
            "p95_engine_ms",
            "p95_work_ms",
            "p95_gpu_ms",
            "max_rss_mb",
        ] {
            let Some(base_value) = parse_metric(&base, metric) else {
                continue;
            };
            let Some(current_value) = parse_metric(&candidate, metric) else {
                eprintln!("[spot][perf-compare] {scenario} {metric} MISSING in candidate");
                regressions += 1;
                continue;
            };
            if report_comparison(scenario, metric, base_value, current_value, tolerance) {
                regressions += 1;
            }
        }

        let baseline_gpu_samples = parse_metric(&base, "gpu_samples").unwrap_or(0.0);
        let current_gpu_samples = parse_metric(&candidate, "gpu_samples").unwrap_or(0.0);
        let current_samples = parse_metric(&candidate, "samples").unwrap_or(0.0);
        if baseline_gpu_samples > 0.0
            && current_samples > 0.0
            && current_gpu_samples / current_samples < 0.90
        {
            eprintln!(
                "[spot][perf-compare] {scenario} GPU coverage {current_gpu_samples:.0}/{current_samples:.0} is below 90% REGRESSION"
            );
            regressions += 1;
        }
    }

    let baseline_cpu = baseline.join("cpu.csv");
    let current_cpu = current.join("cpu.csv");
    if baseline_cpu.exists() {
        if !current_cpu.exists() {
            eprintln!("[spot][perf-compare] MISSING {}", current_cpu.display());
            regressions += 1;
        } else {
            let base_rows = read_keyed_csv(&baseline_cpu, "benchmark")?;
            let current_rows = read_keyed_csv(&current_cpu, "benchmark")?;
            for (benchmark, base) in base_rows {
                let Some(candidate) = current_rows.get(&benchmark) else {
                    eprintln!("[spot][perf-compare] MISSING cpu benchmark {benchmark}");
                    regressions += 1;
                    continue;
                };
                if let (Some(base_value), Some(current_value)) = (
                    parse_metric(&base, "p95_ns"),
                    parse_metric(candidate, "p95_ns"),
                ) && report_comparison(
                    &format!("cpu/{benchmark}"),
                    "p95_ns",
                    base_value,
                    current_value,
                    tolerance,
                ) {
                    regressions += 1;
                }
            }
        }
    }

    if regressions > 0 {
        return Err(format!(
            "{regressions} regression(s) exceeded the {tolerance:.1}% tolerance"
        ));
    }
    eprintln!("[spot][perf-compare] PASS: no regression exceeded {tolerance:.1}%");
    Ok(())
}

fn report_comparison(
    scenario: &str,
    metric: &str,
    baseline: f64,
    current: f64,
    tolerance: f64,
) -> bool {
    let change = if baseline.abs() < f64::EPSILON {
        0.0
    } else {
        (current / baseline - 1.0) * 100.0
    };
    let regression = change > tolerance;
    eprintln!(
        "[spot][perf-compare] {} {:<24} baseline={:.3} current={:.3} change={:+.1}% {}",
        scenario,
        metric,
        baseline,
        current,
        change,
        if regression { "REGRESSION" } else { "ok" }
    );
    regression
}

fn summary_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".summary.csv"))
        })
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn read_single_row_csv(path: &Path) -> Result<HashMap<String, String>, String> {
    let rows = read_csv(path)?;
    rows.into_iter()
        .next()
        .ok_or_else(|| format!("{} has no data rows", path.display()))
}

fn read_keyed_csv(
    path: &Path,
    key: &str,
) -> Result<HashMap<String, HashMap<String, String>>, String> {
    let mut output = HashMap::new();
    for row in read_csv(path)? {
        let value = row
            .get(key)
            .ok_or_else(|| format!("{} is missing {key}", path.display()))?
            .clone();
        output.insert(value, row);
    }
    Ok(output)
}

fn read_csv(path: &Path) -> Result<Vec<HashMap<String, String>>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut lines = content.lines();
    let headers = split_csv_line(
        lines
            .next()
            .ok_or_else(|| format!("{} is empty", path.display()))?,
    );
    Ok(lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| headers.iter().cloned().zip(split_csv_line(line)).collect())
        .collect())
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => fields.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    fields.push(current);
    fields
}

fn parse_metric(row: &HashMap<String, String>, metric: &str) -> Option<f64> {
    row.get(metric)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_parser_handles_quoted_commas_and_quotes() {
        assert_eq!(
            split_csv_line("\"a,b\",\"c\"\"d\",1"),
            vec!["a,b", "c\"d", "1"]
        );
    }
}
