use rand::prelude::*;
use shell_escape::escape;
use std::borrow::Cow;
use std::env;
use std::ffi::OsString;
use std::process::{exit, Command, Stdio};
use std::time::Instant;

fn main() {
    let mut parameters: Vec<String> = vec![];
    let mut template: Vec<String> = vec![];
    let mut programs: Vec<Vec<String>> = vec![];

    for (i, arg) in env::args().enumerate() {
        if i == 0 {
            continue;
        }
        if i == 1 {
            // Parse parameter list
            parameters = arg.split(',').map(|s| s.to_owned()).collect();
            for _ in parameters.iter() {
                programs.push(vec![]);
            }
            continue;
        }
        template.push(arg.to_owned());
        // Accumulate program args, once for each parameter.
        // If we see {} then replace that with the parameter itself.
        for (j, param) in parameters.iter().enumerate() {
            if arg == "{}" {
                programs[j].push(param.to_owned());
            } else {
                programs[j].push(arg.to_owned());
            }
        }
    }

    if template.is_empty() {
        eprintln!("error: missing program to run");
        eprintln!(
            "usage: ab opt-A,opt-B my-program {{}} # opt-A and opt-B will get substituted for {{}}"
        );
        std::process::exit(1);
    }

    // Warm up each program.
    for p in &programs {
        // let mut executable = OsString::new();
        // executable.push(p[0]);
        eprintln!("warming up: {}", shlex_quote(p));
        let mut cmd = get_command(&p);
        let mut child = cmd.spawn().unwrap_or_else(|e| {
            eprintln!("warmup: command failed: {}", e);
            exit(1);
        });
        let status = child.wait().unwrap_or_else(|e| {
            eprintln!("warmup: wait() failed: {}", e);
            exit(1);
        });
        if !status.success() {
            eprintln!("warmup: failed");
            exit(1);
        }
    }

    let mut rng = rand::thread_rng();

    // Now run the programs, alternating randomly.
    let mut durations: Vec<Vec<f64>> = programs.iter().map(|_| vec![]).collect();
    for r in 1..=10000 {
        let i = rng.gen_range(0..programs.len());
        let p = &programs[i];
        let mut cmd = get_command(&p);
        // eprintln!("{}", parameters[i]);
        let start = Instant::now();
        let mut child = cmd.spawn().unwrap_or_else(|e| {
            eprintln!("command failed: {}", e);
            exit(1);
        });
        let status = child.wait().unwrap_or_else(|e| {
            eprintln!("wait() failed: {}", e);
            exit(1);
        });
        let duration = start.elapsed();
        if !status.success() {
            eprintln!("command failed");
            exit(1);
        }

        insert_sorted(&mut durations[i], duration.as_secs_f64());

        let mut best = -1.0;
        let mut max = 0.0;
        for i in 0..programs.len() {
            let avg = durations[i].iter().sum::<f64>() / (durations[i].len() as f64);
            if best < 0.0 || avg < best {
                best = avg;
            }
            if durations[i].len() > 0 {
                let top = durations[i][durations[i].len() - 1];
                if top > max {
                    max = top;
                }
            }
        }

        let mut hist: Vec<Vec<usize>> = vec![];
        let mut max_count: usize = 0;
        for samples in &durations {
            // TODO: use actual min instead of 0.0?
            let counts = hist_buckets(&samples, 100, 0.0, max);
            if let Some(m) = counts.iter().max() {
                let m = m.to_owned();
                if m > max_count {
                    max_count = m;
                }
            }
            hist.push(counts);
        }

        // Print results so far
        eprint!("\x1b[2J\x1b[H"); // clear terminal
        eprint!("{}\n\n", shlex_quote(&template));
        for i in 0..programs.len() {
            let avg = durations[i].iter().sum::<f64>() / (durations[i].len() as f64);

            let mut extras = "".to_owned();
            if r >= 20 {
                let mut rel = "(best)".to_owned();
                if avg != best {
                    rel = format!("+{:.1}%", 100.0 * ((avg / best) - 1.0));
                }
                extras = format!(
                    "\t{}\tp50 {:.4}s\tp95 {:.4}s\tp99 {:.4}s",
                    rel,
                    quantile(&durations[i], 0.5),
                    quantile(&durations[i], 0.95),
                    quantile(&durations[i], 0.99),
                );
            }

            eprintln!(
                "{}\t{:.4}s\tN={}{}",
                parameters[i],
                avg,
                durations[i].len(),
                extras
            );

            // Print heatmap
            for c in &hist[i] {
                let color_black = 232;
                let color_white = 255;
                let brightness_max = color_white - color_black + 1;
                let brightness =
                    ((c.to_owned() as f64 / max_count as f64) * (brightness_max as f64)) as i32;
                let color = color_black + brightness;
                eprint!("\x1b[48;5;{};{};{}m \x1b[0m", color, color, color);
            }
            eprint!("\n");
        }
    }
}

fn hist_buckets(samples: &Vec<f64>, n: usize, min: f64, max: f64) -> Vec<usize> {
    let mut counts = vec![0; n];
    for x in samples {
        let mut idx = (((x - min) / (max - min)) * (n as f64)) as usize;
        // TODO: avoid this hack
        if idx == counts.len() {
            idx = counts.len() - 1;
        }
        counts[idx] += 1;
    }
    return counts;
}

fn insert_sorted(vec: &mut Vec<f64>, val: f64) {
    vec.push(val);
    let mut i = vec.len() - 1;
    while i > 0 {
        if vec[i] > vec[i - 1] {
            return;
        }
        vec.swap(i, i - 1);
        i -= 1;
    }
}

fn quantile(vec: &Vec<f64>, q: f64) -> f64 {
    let index = ((vec.len() as f64) * q) as usize;
    return vec[index];
}

fn get_command(p: &Vec<String>) -> Command {
    let mut cmd = Command::new(OsString::from(p[0].to_owned()));
    for arg in &p[1..] {
        cmd.arg(arg);
    }
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    return cmd;
}

fn shlex_quote(args: &Vec<String>) -> String {
    args.iter()
        .map(|arg| {
            if arg == "{}" {
                arg.to_owned()
            } else {
                escape(Cow::from(arg)).to_string()
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}
