use clap::{Arg, ArgAction, Command};
use console::{Emoji, Style};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::{fs, io::{self, Write}, path::Path, time::Duration};
use walkdir::WalkDir;
use log::{debug, error, info};

// Emoji constants
static WARN: Emoji = Emoji("⚠️ ", "!");
static TRASH: Emoji = Emoji("🗑 ", "-");
static MAG: Emoji = Emoji("🔍", "*");
static DISK: Emoji = Emoji("💾", ">");
static GEAR: Emoji = Emoji("⚙️ ", ">");
static TICK: Emoji = Emoji("✅", "+");
static CROSS: Emoji = Emoji("❌", "x");
static INFO: Emoji = Emoji("ℹ️ ", "i");

// Color styles - Fixed the color() method issue
fn cyan() -> Style { Style::new().cyan() }
fn green() -> Style { Style::new().green() }
fn red() -> Style { Style::new().red() }
fn yellow() -> Style { Style::new().yellow() }
fn bold() -> Style { Style::new().bold() }

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    target: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    depth: Option<usize>,
    min_size: Option<f64>,
    min_age: Option<i64>,
    follow_symlinks: Option<bool>,
    delete: Option<bool>,
    yes: Option<bool>,
    dry_run: Option<bool>,
    use_trash: Option<bool>,
    backup: Option<bool>,
    archive: Option<bool>,
    backup_dir: Option<String>,
    interactive: Option<bool>,
    confirm_phrase: Option<String>,
    json: Option<String>,
    csv: Option<String>,
    log: Option<String>,
    verbose: Option<bool>,
    quiet: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DirInfo {
    path: String,
    size_bytes: u64,
    age_days: Option<i64>,
    item_count: Option<usize>,
}

fn load_config(config_path: &str) -> Result<Config, String> {
    debug!("Loading config from {}", config_path);
    fs::read_to_string(config_path)
        .map_err(|e| format!("{} Error reading config: {}", CROSS, e))
        .and_then(|content| serde_json::from_str(&content)
        .map_err(|e| format!("{} Error parsing config: {}", CROSS, e)))
}

fn save_config(config: &Config, config_path: &str) -> Result<(), String> {
    debug!("Saving config to {}", config_path);
    serde_json::to_string_pretty(config)
        .map_err(|e| format!("{} Error serializing config: {}", CROSS, e))
        .and_then(|content| fs::write(config_path, content)
        .map_err(|e| format!("{} Error writing config: {}", CROSS, e)))
}

fn get_directory_size(path: &Path, follow_symlinks: bool) -> u64 {
    WalkDir::new(path)
        .follow_links(follow_symlinks)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .fold(0, |acc, m| acc + m.len())
}

fn count_directory_items(path: &Path, follow_symlinks: bool) -> usize {
    WalkDir::new(path)
        .follow_links(follow_symlinks)
        .into_iter()
        .filter_map(|e| e.ok())
        .count()
}

fn directory_modified_days_ago(path: &Path) -> Option<i64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .elapsed()
        .ok()
        .map(|d| d.as_secs() as i64 / 86400)
}

fn find_directories(
    base_path: &str,
    target: &[String],
    exclude: &[String],
    depth: Option<usize>,
    min_size: Option<u64>,
    min_age: Option<i64>,
    follow_symlinks: bool,
    verbose: bool,
) -> Vec<DirInfo> {
    let base = Path::new(base_path);
    
    // Create a progress bar for directory scanning if verbose
    let spinner = if verbose {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner} Scanning directories... {elapsed_precise}")
                .unwrap()
        );
        sp.enable_steady_tick(Duration::from_millis(100));
        Some(sp)
    } else {
        None
    };

    // Set up the walker with depth if specified
    let walker = match depth {
        Some(d) => WalkDir::new(base).max_depth(d),
        None => WalkDir::new(base)
    };

    let result = walker.into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            let path_str = e.path().to_string_lossy();
            
            // Skip directory if it's in the exclude list
            if exclude.iter().any(|ex| path_str.contains(ex)) {
                debug!("Excluding directory: {}", path_str);
                return false;
            }
            
            // Include directory if it's in the target list
            let matches = target.iter().any(|t| name.contains(t));
            if matches && verbose {
                debug!("Found matching directory: {}", path_str);
            }
            matches
        })
        .filter(|e| {
            min_age.map_or(true, |min| {
                directory_modified_days_ago(e.path())
                    .map_or(false, |age| age >= min)
            })
        })
        .filter_map(|e| {
            if let Some(spinner) = &spinner {
                spinner.set_message(format!("Analyzing {}", e.path().display()));
            }
            
            let size = get_directory_size(e.path(), follow_symlinks);
            let age = directory_modified_days_ago(e.path());
            let item_count = Some(count_directory_items(e.path(), follow_symlinks));
            
            min_size.map_or(Some(size), |min| (size >= min).then_some(size))
                .map(|size| DirInfo {
                    path: e.path().to_string_lossy().into_owned(),
                    size_bytes: size,
                    age_days: age,
                    item_count,
                })
        })
        .collect::<Vec<_>>();
    
    // Finish and clear the spinner
    if let Some(spinner) = spinner {
        spinner.finish_and_clear();
    }
    
    result
}

fn archive_directory(path: &str, backup_dir: &str) -> Result<String, String> {
    let dir_path = Path::new(path);
    let backup_path = Path::new(backup_dir);
    
    fs::create_dir_all(backup_path)
        .map_err(|e| format!("{} Failed to create backup directory: {}", CROSS, e))?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let archive_name = format!("{}_{}.zip",
        dir_path.file_name()
            .ok_or_else(|| format!("{} Invalid directory name", CROSS))?
            .to_string_lossy(),
        timestamp
    );
    
    let archive_path = backup_path.join(&archive_name);
    let archive_file = fs::File::create(&archive_path)
        .map_err(|e| format!("{} Failed to create archive file: {}", CROSS, e))?;
    
    let mut zip = zip::ZipWriter::new(archive_file);
    
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    
    let mut buffer = Vec::new();
    
    // Walk the directory and add all files to the zip
    let walker = WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok());
    
    for entry in walker {
        let path = entry.path();
        let name = path.strip_prefix(Path::new(path))
            .unwrap_or(path)
            .to_string_lossy();
        
        if path.is_file() {
            debug!("Adding to archive: {}", name);
            zip.start_file(name.to_string(), options)
                .map_err(|e| format!("{} Failed to add file to archive: {}", CROSS, e))?;
            
            let mut f = fs::File::open(path)
                .map_err(|e| format!("{} Failed to open file for archiving: {}", CROSS, e))?;
            
            io::copy(&mut f, &mut buffer)
                .map_err(|e| format!("{} Failed to read file for archiving: {}", CROSS, e))?;
            
            zip.write_all(&buffer)
                .map_err(|e| format!("{} Failed to write file to archive: {}", CROSS, e))?;
            
            buffer.clear();
        } else if !path.as_os_str().is_empty() {
            // Only create explicit directory entries for non-root directories
            zip.add_directory(name.to_string(), options)
                .map_err(|e| format!("{} Failed to add directory to archive: {}", CROSS, e))?;
        }
    }
    
    zip.finish()
        .map_err(|e| format!("{} Failed to finalize archive: {}", CROSS, e))?;
    
    Ok(archive_path.to_string_lossy().to_string())
}

fn backup_directory(path: &str, backup_dir: &str) -> Result<String, String> {
    let dir_path = Path::new(path);
    let backup_root = Path::new(backup_dir);
    
    fs::create_dir_all(backup_root)
        .map_err(|e| format!("{} Failed to create backup directory: {}", CROSS, e))?;
    
    let dir_name = dir_path.file_name()
        .ok_or_else(|| format!("{} Invalid directory name", CROSS))?;
        
    let backup_path = backup_root.join(dir_name);
    
    if backup_path.exists() {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let new_backup_path = backup_root.join(format!(
            "{}_{}", 
            dir_name.to_string_lossy(),
            timestamp
        ));
        
        debug!("Backup destination already exists, creating timestamped backup: {}", new_backup_path.display());
        
        // Use copy_dir instead of fs::copy for directories
        copy_dir_recursive(dir_path, &new_backup_path)
            .map_err(|e| format!("{} Backup failed: {}", CROSS, e))?;
            
        return Ok(new_backup_path.to_string_lossy().to_string());
    }
    
    // Use copy_dir instead of fs::copy for directories
    copy_dir_recursive(dir_path, &backup_path)
        .map_err(|e| format!("{} Backup failed: {}", CROSS, e))?;

    Ok(backup_path.to_string_lossy().to_string())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if ty.is_file() {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn delete_directories(
    dirs: &[DirInfo],
    dry_run: bool,
    verbose: bool,
    use_trash: bool,
    backup: bool,
    archive: bool,
    backup_dir: Option<&str>,
    interactive: bool,
) -> Result<Vec<String>, String> {
    let pb = ProgressBar::new(dirs.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("🟩🟧🟥")
    );

    let mut processed_paths = Vec::new();
    let mut backup_paths = Vec::new();

    for dir in dirs {
        pb.inc(1);
        
        // Interactive mode - ask for confirmation for each directory
        if interactive && !dry_run {
            println!("\n{} Directory: {}", INFO, bold().apply_to(&dir.path));
            println!("   Size: {:.2} MB", dir.size_bytes as f64 / 1024.0 / 1024.0);
            if let Some(age) = dir.age_days {
                println!("   Age: {} days", age);
            }
            if let Some(count) = dir.item_count {
                println!("   Items: {}", count);
            }
            
            print!("{} Delete this directory? (y/n): ", WARN);
            io::stdout().flush().map_err(|e| format!("IO error: {}", e))?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)
                .map_err(|e| format!("{} Input error: {}", CROSS, e))?;
                
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("{} Skipping directory", INFO);
                continue;
            }
        }
        
        // Handle backup or archive if requested
        if (backup || archive) && backup_dir.is_some() {
            let backup_dir = backup_dir.unwrap();
            let result = if archive {
                archive_directory(&dir.path, backup_dir)
            } else {
                backup_directory(&dir.path, backup_dir)
            };
            
            match result {
                Ok(path) => {
                    if verbose {
                        println!("{} {}", 
                            DISK,
                            green().apply_to(format!("{} to: {}", 
                                if archive { "Archived" } else { "Backed up" }, 
                                path
                            ))
                        );
                    }
                    backup_paths.push(path);
                },
                Err(e) => {
                    pb.abandon_with_message(format!("{} Operation failed", CROSS));
                    return Err(e);
                }
            }
        }

        if !dry_run {
            match handle_deletion(&dir.path, use_trash, verbose) {
                Ok(_) => processed_paths.push(dir.path.clone()),
                Err(e) => {
                    pb.abandon_with_message(format!("{} Operation failed", CROSS));
                    return Err(e);
                }
            }
        } else if verbose {
            println!("{} {}", 
                yellow().apply_to(WARN),
                cyan().apply_to(format!("[Dry Run] Would delete: {}", dir.path))
            );
            processed_paths.push(dir.path.clone());
        }
    }
    
    pb.finish_with_message(format!("{} {}", 
        green().apply_to(TICK),
        green().apply_to("Operation completed successfully!")
    ));
    
    Ok(backup_paths)
}

fn handle_deletion(path: &str, use_trash: bool, verbose: bool) -> Result<(), String> {
    if use_trash {
        match trash::delete(path) {
            Ok(_) => {
                if verbose {
                    println!("{} {}", 
                        TRASH,
                        green().apply_to(format!("Moved to trash: {}", path))
                    );
                }
                Ok(())
            },
            Err(e) => {
                error!("Trash operation failed for {}: {}", path, e);
                Err(format!("{} Trash failed: {}", CROSS, e))
            }
        }
    } else {
        match fs::remove_dir_all(path) {
            Ok(_) => {
                if verbose {
                    println!("{} {}", 
                        CROSS,
                        red().apply_to(format!("Permanently deleted: {}", path))
                    );
                }
                Ok(())
            },
            Err(e) => {
                error!("Deletion failed for {}: {}", path, e);
                Err(format!("{} Deletion failed: {}", CROSS, e))
            }
        }
    }
}

fn export_summary(
    dirs: &[DirInfo], 
    json_path: Option<&str>, 
    csv_path: Option<&str>,
    backup_paths: &[String],
) -> Result<(), String> {
    // Create a summary object with more details
    #[derive(Serialize)]
    struct Summary {
        directories: Vec<DirInfo>,
        total_size_bytes: u64,
        total_size_mb: f64,
        count: usize,
        average_size_mb: f64,
        oldest_dir_days: Option<i64>,
        newest_dir_days: Option<i64>,
        backups: Vec<String>,
        timestamp: String,
    }
    
    let total_size: u64 = dirs.iter().map(|d| d.size_bytes).sum();
    let total_size_mb = total_size as f64 / 1024.0 / 1024.0;
    let average_size_mb = if !dirs.is_empty() { total_size_mb / dirs.len() as f64 } else { 0.0 };
    
    let oldest_dir_days = dirs.iter()
        .filter_map(|d| d.age_days)
        .max();
        
    let newest_dir_days = dirs.iter()
        .filter_map(|d| d.age_days)
        .min();
    
    let summary = Summary {
        directories: dirs.to_vec(),
        total_size_bytes: total_size,
        total_size_mb,
        count: dirs.len(),
        average_size_mb,
        oldest_dir_days,
        newest_dir_days,
        backups: backup_paths.to_vec(),
        timestamp: chrono::Local::now().to_rfc3339(),
    };

    if let Some(json_file) = json_path {
        match serde_json::to_string_pretty(&summary) {
            Ok(json) => {
                if let Err(e) = fs::write(json_file, json) {
                    error!("JSON export error: {}", e);
                    eprintln!("{} {}", 
                        CROSS,
                        red().apply_to(format!("JSON export error: {}", e))
                    );
                } else {
                    info!("Saved JSON summary to {}", json_file);
                    println!("{} {}", 
                        DISK,
                        green().apply_to(format!("Saved JSON summary to {}", json_file))
                    );
                }
            }
            Err(e) => {
                error!("JSON serialization error: {}", e);
                eprintln!("{} {}", 
                    CROSS,
                    red().apply_to(format!("JSON serialization error: {}", e))
                );
            }
        }
    }
    
    if let Some(csv_file) = csv_path {
        match csv::Writer::from_path(csv_file) {
            Ok(mut wtr) => {
                if let Err(e) = dirs.iter().try_for_each(|d| wtr.serialize(d)) {
                    error!("CSV export error: {}", e);
                    eprintln!("{} {}", 
                        CROSS,
                        red().apply_to(format!("CSV export error: {}", e))
                    );
                } else {
                    info!("Saved CSV summary to {}", csv_file);
                    println!("{} {}", 
                        DISK,
                        green().apply_to(format!("Saved CSV summary to {}", csv_file))
                    );
                }
            }
            Err(e) => {
                error!("CSV creation error: {}", e);
                eprintln!("{} {}", 
                    CROSS,
                    red().apply_to(format!("CSV creation error: {}", e))
                );
            }
        }
    }
    
    Ok(())
}

fn confirm_deletion(phrase: Option<&String>) -> Result<bool, String> {
    let default_phrase = "DELETE".to_string();
    let phrase = phrase.unwrap_or(&default_phrase);
    
    println!("{} {}",
        yellow().apply_to(WARN),
        red().apply_to("WARNING! This will permanently delete directories!")
    );
    println!("{} Type '{}' to confirm:",
        yellow().apply_to("⚠️ "),
        cyan().apply_to(phrase)
    );
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .map_err(|e| format!("{} Input error: {}", CROSS, e))?;

    Ok(input.trim() == phrase)
}

fn interactive_select_directories(dirs: &[DirInfo]) -> Vec<DirInfo> {
    println!("{} {}", INFO, bold().apply_to("Select directories to delete:"));
    println!("{} Press y/n for each directory, or 'a' to select all, 'q' to quit", INFO);
    
    let mut selected = Vec::new();
    let mut select_all = false;
    
    for (i, dir) in dirs.iter().enumerate() {
        if select_all {
            selected.push(dir.clone());
            println!("[{}/{}] ✅ Selected: {}", i+1, dirs.len(), dir.path);
            continue;
        }
        
        println!("\n[{}/{}] Directory: {}", i+1, dirs.len(), bold().apply_to(&dir.path));
        println!("   Size: {:.2} MB", dir.size_bytes as f64 / 1024.0 / 1024.0);
        if let Some(age) = dir.age_days {
            println!("   Age: {} days", age);
        }
        if let Some(count) = dir.item_count {
            println!("   Items: {}", count);
        }
        
        print!("Select? (y/n/a/q): ");
        io::stdout().flush().unwrap_or(());
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        
        match input.trim().to_lowercase().as_str() {
            "y" => {
                selected.push(dir.clone());
                println!("✅ Selected");
            },
            "a" => {
                select_all = true;
                selected.push(dir.clone());
                println!("✅ Selected all remaining directories");
            },
            "q" => {
                println!("🛑 Selection canceled");
                break;
            },
            _ => println!("❌ Skipped"),
        }
    }
    
    selected
}

fn setup_logger(log_file: Option<&str>, verbose: bool) -> Result<(), String> {
    let mut builder = env_logger::Builder::new();
    
    // Set log level based on verbose flag
    builder.filter_level(if verbose { 
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    });
    
    // Format for standard output
    builder.format_timestamp(None);
    builder.format_module_path(false);
    
    // Add file logger if specified
    if let Some(log_path) = log_file {
        let file = fs::File::create(log_path)
            .map_err(|e| format!("{} Failed to create log file: {}", CROSS, e))?;
            
        builder.target(env_logger::Target::Pipe(Box::new(file)));
    }
    
    builder.init();
    
    Ok(())
}

fn main() -> Result<(), String> {
    let matches = Command::new("🧹 dirpurge")
        .version("1.0.0")
        .about("Advanced directory cleanup tool with safety features")
        .help_template(
            "{before-help}{name} {version}\n{author-with-newline}{about-with-newline}\n{usage-heading} {usage}\n\n{all-args}{after-help}"
        )
        .arg(Arg::new("path")
            .help("📁 Base directory to search")
            .required(true)
            .index(1))
        .arg(Arg::new("target")
            .short('t')
            .long("target")
            .help("🔎 Directory names to search for (multiple allowed)")
            .action(ArgAction::Append)
            .value_parser(clap::builder::NonEmptyStringValueParser::new())
            .default_values(["venv", ".venv", "node_modules", "target", "bin", "build"]))
        .arg(Arg::new("exclude")
            .short('e')
            .long("exclude")
            .help("🚫 Directories to exclude from search")
            .action(ArgAction::Append))
        .arg(Arg::new("depth")
            .long("depth")
            .help("📏 Maximum search depth (0 = unlimited)")
            .value_parser(clap::value_parser!(usize)))
        .arg(Arg::new("min-size")
            .long("min-size")
            .help("📦 Minimum directory size in MB to include")
            .value_parser(clap::value_parser!(f64)))
        .arg(Arg::new("min-age")
            .long("min-age")
            .help("📅 Minimum age in days to include")
            .value_parser(clap::value_parser!(i64)))
        .arg(Arg::new("follow-symlinks")
            .long("follow-symlinks")
            .help("🔗 Follow symbolic links during search")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("delete")
            .long("delete")
            .help(format!("{} Perform deletion", TRASH))
            .action(ArgAction::SetTrue))
        .arg(Arg::new("yes")
            .short('y')
            .long("yes")
            .help("✅ Skip confirmation prompts")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("dry-run")
            .short('d')
            .long("dry-run")
            .help("🌵 Simulate operations without making changes")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("use-trash")
            .long("use-trash")
            .help("🗑  Move to trash instead of permanent deletion")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("backup")
            .short('b')
            .long("backup")
            .help("💾 Create backups before deletion")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("archive")
            .short('a')
            .long("archive")
            .help("📦 Create zip archives before deletion")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("backup-dir")
            .long("backup-dir")
            .help("📂 Directory for backups/archives")
            .value_name("DIR")
            .default_value("./backups"))
        .arg(Arg::new("interactive")
            .short('i')
            .long("interactive")
            .help("🖱  Select directories to delete interactively")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("confirm-phrase")
            .long("confirm-phrase")
            .help("🔐 Custom confirmation phrase for deletion")
            .default_value("DELETE"))
        .arg(Arg::new("json")
            .long("json")
            .help("📄 Export results to JSON file")
            .value_name("FILE"))
        .arg(Arg::new("csv")
            .long("csv")
            .help("📊 Export results to CSV file")
            .value_name("FILE"))
        .arg(Arg::new("log")
            .long("log")
            .help("📝 Write log to file")
            .value_name("FILE"))
        .arg(Arg::new("config")
            .short('c')
            .long("config")
            .help("⚙️  Load configuration from JSON file")
            .value_name("FILE"))
        .arg(Arg::new("save-config")
            .long("save-config")
            .help("💾 Save current settings to config file")
            .value_name("FILE"))
        .arg(Arg::new("verbose")
            .short('v')
            .long("verbose")
            .help("🔊 Enable verbose output")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("quiet")
            .short('q')
            .long("quiet")
            .help("🔈 Suppress non-essential output")
            .action(ArgAction::SetTrue))
        .after_help(format!(
            "{}\n{}{}",
            yellow().apply_to("💡 Tip: Always run with --dry-run first to test!"),
            cyan().apply_to("\nExamples:\n  "),
            cyan().apply_to("dirpurge ./project\n  dirpurge ./src -t node_modules --delete\n  dirpurge . --config settings.json\n  dirpurge . -i --use-trash")
        ))
        .get_matches();

    // Set up logging
    setup_logger(
        matches.get_one::<String>("log").map(String::as_str),
        matches.get_flag("verbose")
    )?;

    // Load config file if specified
    let mut config = matches.get_one::<String>("config")
        .and_then(|config_path| load_config(config_path).ok())
        .unwrap_or_else(|| Config {
            target: None,
            exclude: None,
            depth: None,
            min_size: None,
            min_age: None,
            follow_symlinks: None,
            delete: None,
            yes: None,
            dry_run: None,
            use_trash: None,
            backup: None,
            archive: None,
            backup_dir: None,
            interactive: None,
            confirm_phrase: None,
            json: None,
            csv: None,
            log: None,
            verbose: None,
            quiet: None,
        });

    // Base path is required
    let base_path = matches.get_one::<String>("path").unwrap();

    // Get command line args and override config values
    if let Some(targets) = matches.get_many::<String>("target") {
        config.target = Some(targets.cloned().collect());
    }
    if let Some(excludes) = matches.get_many::<String>("exclude") {
        config.exclude = Some(excludes.cloned().collect());
    }
    if let Some(depth) = matches.get_one::<usize>("depth") {
        config.depth = Some(*depth);
    }
    if let Some(min_size) = matches.get_one::<f64>("min-size") {
        config.min_size = Some(*min_size);
    }
    if let Some(min_age) = matches.get_one::<i64>("min-age") {
        config.min_age = Some(*min_age);
    }
    if matches.contains_id("follow-symlinks") {
        config.follow_symlinks = Some(matches.get_flag("follow-symlinks"));
    }
    if matches.contains_id("delete") {
        config.delete = Some(matches.get_flag("delete"));
    }
    if matches.contains_id("yes") {
        config.yes = Some(matches.get_flag("yes"));
    }
    if matches.contains_id("dry-run") {
        config.dry_run = Some(matches.get_flag("dry-run"));
    }
    if matches.contains_id("use-trash") {
        config.use_trash = Some(matches.get_flag("use-trash"));
    }
    if matches.contains_id("backup") {
        config.backup = Some(matches.get_flag("backup"));
    }
    if matches.contains_id("archive") {
        config.archive = Some(matches.get_flag("archive"));
    }
    if let Some(backup_dir) = matches.get_one::<String>("backup-dir") {
        config.backup_dir = Some(backup_dir.clone());
    }
    if matches.contains_id("interactive") {
        config.interactive = Some(matches.get_flag("interactive"));
    }
    if let Some(confirm_phrase) = matches.get_one::<String>("confirm-phrase") {
        config.confirm_phrase = Some(confirm_phrase.clone());
    }
    if let Some(json) = matches.get_one::<String>("json") {
        config.json = Some(json.clone());
    }
    if let Some(csv) = matches.get_one::<String>("csv") {
        config.csv = Some(csv.clone());
    }
    if let Some(log_file) = matches.get_one::<String>("log") {
        config.log = Some(log_file.clone());
    }
    if matches.contains_id("verbose") {
        config.verbose = Some(matches.get_flag("verbose"));
    }
    if matches.contains_id("quiet") {
        config.quiet = Some(matches.get_flag("quiet"));
    }

    // Save config if requested
    if let Some(config_path) = matches.get_one::<String>("save-config") {
        save_config(&config, config_path)?;
        println!("{} {}", DISK, green().apply_to(format!("Configuration saved to {}", config_path)));
    }

    // Extract config values with defaults
    let target = config.target.clone().unwrap_or_else(|| vec!["venv".to_string(), ".venv".to_string(), "node_modules".to_string()]);
    let exclude = config.exclude.clone().unwrap_or_default();
    let depth = config.depth;
    let min_size = config.min_size.map(|mb| (mb * 1024.0 * 1024.0) as u64);
    let min_age = config.min_age;
    let follow_symlinks = config.follow_symlinks.unwrap_or(false);
    let delete_enabled = config.delete.unwrap_or(false);
    let yes = config.yes.unwrap_or(false);
    let dry_run = config.dry_run.unwrap_or(false);
    let use_trash = config.use_trash.unwrap_or(true);
    let backup = config.backup.unwrap_or(false);
    let archive = config.archive.unwrap_or(false);
    let backup_dir = config.backup_dir.clone().unwrap_or_else(|| "./backups".to_string());
    let interactive = config.interactive.unwrap_or(false);
    let confirm_phrase = config.confirm_phrase.clone();
    let json_output = config.json.clone();
    let csv_output = config.csv.clone();
    let verbose = config.verbose.unwrap_or(false);
    let quiet = config.quiet.unwrap_or(false);

    // Show banner and configuration summary
    if !quiet {
        println!("\n{} {} v1.0.0", GEAR, bold().apply_to("🧹 dirpurge"));
        println!("{} {}", MAG, cyan().apply_to(format!("Searching in: {}", base_path)));
        println!("{} {}", MAG, cyan().apply_to(format!("Targets: {}", target.join(", "))));
        
        if !exclude.is_empty() {
            println!("{} {}", MAG, cyan().apply_to(format!("Excluding: {}", exclude.join(", "))));
        }
        
        if verbose {
            println!("{} {}", MAG, cyan().apply_to(format!("Depth: {}", depth.map_or("unlimited".to_string(), |d| d.to_string()))));
            println!("{} {}", MAG, cyan().apply_to(format!("Min size: {}", min_size.map_or("none".to_string(), |s| format!("{:.2} MB", s as f64 / 1024.0 / 1024.0)))));
            println!("{} {}", MAG, cyan().apply_to(format!("Min age: {}", min_age.map_or("none".to_string(), |a| format!("{} days", a)))));
            println!("{} {}", MAG, cyan().apply_to(format!("Follow symlinks: {}", follow_symlinks)));
            println!("{} {}", MAG, cyan().apply_to(format!("Mode: {}", if dry_run { "DRY RUN" } else if delete_enabled { "DELETE" } else { "SCAN ONLY" })));
        }
    }

    // Find matching directories
    let mut dirs = find_directories(
        base_path,
        &target,
        &exclude,
        depth,
        min_size,
        min_age,
        follow_symlinks,
        verbose,
    );
    
    // Sort directories by size (largest first)
    dirs.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    // Handle when no matching directories are found
    if dirs.is_empty() {
        info!("No matching directories found");
        println!("{} {}", INFO, yellow().apply_to("No matching directories found"));
        return Ok(());
    }

    // Show found directories
    if !quiet {
        println!("\n{} {} matching directories found:", TICK, bold().apply_to(dirs.len()));
        
        let total_size: u64 = dirs.iter().map(|d| d.size_bytes).sum();
        println!("{} Total size: {:.2} MB", INFO, total_size as f64 / 1024.0 / 1024.0);
        
        for (i, dir) in dirs.iter().enumerate().take(10) {
            println!("  {}. {} ({:.2} MB)", 
                i + 1,
                dir.path,
                dir.size_bytes as f64 / 1024.0 / 1024.0
            );
        }
        
        if dirs.len() > 10 {
            println!("  ... and {} more", dirs.len() - 10);
        }
    }
    
    // Interactive mode - select directories to delete
    let selected_dirs = if interactive {
        interactive_select_directories(&dirs)
    } else {
        dirs.clone()
    };
    
    // If no directories were selected in interactive mode
    if selected_dirs.is_empty() && interactive {
        println!("{} No directories selected for deletion", INFO);
        return Ok(());
    }
    
    // Backup/delete only if requested
    if delete_enabled || dry_run {
        // Skip confirmation if yes flag is provided
        let confirmed = if yes {
            true
        } else {
            confirm_deletion(confirm_phrase.as_ref())?
        };
        
        if confirmed {
            let backup_paths = delete_directories(
                &selected_dirs,
                dry_run,
                verbose,
                use_trash,
                backup,
                archive,
                Some(backup_dir.as_str()),
                false // Interactive selection already done
            )?;
            
            // Export summary if requested
            if json_output.is_some() || csv_output.is_some() {
                export_summary(
                    &selected_dirs,
                    json_output.as_deref(),
                    csv_output.as_deref(),
                    &backup_paths,
                )?;
            }
        } else {
            println!("{} {}", INFO, yellow().apply_to("Operation canceled"));
            return Ok(());
        }
    } else if !quiet {
        println!("\n{} {}", 
            INFO,
            yellow().apply_to("Use --delete to remove directories or --dry-run to simulate")
        );
    }

    Ok(())
}