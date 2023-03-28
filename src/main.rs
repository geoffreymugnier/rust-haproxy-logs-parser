use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;
use std::time::Instant;

use regex::Regex;


fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: log_size_rust <path-to-log-directory>");
        return Ok(());
    }
    
    let log_dir = &args[1];
    let size_pattern = Regex::new(r#""GET /[^"]*\.(?:jpeg|jpg|png) HTTP/1.1" 200 (\d+)"#).unwrap();
    let log_files = get_log_files(log_dir)?;
    let number_of_threads = std::cmp::min(6, log_files.len());
    let total_size = Arc::new(Mutex::new(0));
    let start = Instant::now();

    let pool = ThreadPool::new(number_of_threads);
    spawn_threads(&pool, &log_files, &size_pattern, &total_size)?;

    pool.join();

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

fn spawn_threads(
    pool: &ThreadPool,
    log_files: &[String],
    size_pattern: &Regex,
    total_size: &Arc<Mutex<i64>>,
) -> io::Result<()> {
    for log_file in log_files {
        let total_size = Arc::clone(total_size);
        let size_pattern = size_pattern.clone();
        let log_file = log_file.clone();

        pool.execute(move || {
            let file_size = process_file(&log_file, &size_pattern).expect("Error processing file");
            let mut total_size = total_size.lock().unwrap();
            *total_size += file_size;

        });
    }

    Ok(())
}

fn extract_size_from_line(line: &str, size_pattern: &Regex) -> i64 {
    size_pattern
        .captures(line)
        .and_then(|captures| captures.get(1))
        .and_then(|matched_size| matched_size.as_str().parse::<i64>().ok())
        .unwrap_or(0)
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
        file_size += extract_size_from_line(&line, size_pattern);
    }

    let elapsed_time = start_time.elapsed();
    println!(
        "✅ File {} processed in {:?}",
        log_file.as_ref().display(),
        elapsed_time
    );

    Ok(file_size)
}
