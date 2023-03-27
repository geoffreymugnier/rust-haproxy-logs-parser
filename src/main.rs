use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use regex::Regex;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: log_size_rust <path-to-log-directory>");
        return Ok(());
    }

    let log_dir = &args[1];

    let size_pattern = Regex::new(r#"HTTP/1.1" 200 (\d+) "#).unwrap();
    let log_files = get_log_files(log_dir)?;

    let start = Instant::now();
    let total_size = Arc::new(Mutex::new(0));

    let mut handles = vec![];

    for log_file in log_files {
        let total_size = Arc::clone(&total_size);
        let size_pattern = size_pattern.clone();

        let handle = std::thread::spawn(move || {
            let file_size = process_file(&log_file, &size_pattern).expect("Error processing file");
            let mut total_size = total_size.lock().unwrap();
            *total_size += file_size;
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let total_size = total_size.lock().unwrap();
    let duration = start.elapsed();
    println!("Total size: {:.2} GB",  *total_size as f64 / 1000.0f64.powi(3));
    println!("Time elapsed: {:?}", duration);

    Ok(())
}

fn get_log_files<P: AsRef<Path>>(log_dir: P) -> io::Result<Vec<String>> {
    let mut log_files = vec![];

    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        if entry.path().is_file() {
            if let Some(log_file) = entry.path().to_str() {
                log_files.push(log_file.to_string());
            }
        }
    }

    Ok(log_files)
}

fn process_file<P: AsRef<Path>>(log_file: P, size_pattern: &Regex) -> io::Result<i64> {
    let start_time = Instant::now();
    let file = File::open(&log_file)?;
    let reader = BufReader::new(file);

    println!(
        "🏁 Started processing {}",
        log_file.as_ref().display()
    );

    let mut file_size = 0;

    for line in reader.lines() {
        let line = line?;
        if let Some(captures) = size_pattern.captures(&line) {
            if let Some(matched_size) = captures.get(1) {
                if let Ok(size) = matched_size.as_str().parse::<i64>() {
                    file_size += size;
                }
            }
        }
    }

    let elapsed_time = start_time.elapsed();
    println!(
        "✅ File {} processed in {:?}",
        log_file.as_ref().display(),
        elapsed_time
    );

    Ok(file_size)
}
