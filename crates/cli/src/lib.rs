use std::io::{stdout, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use rustget_core::{
    speed::SpeedTracker, Category, ClipboardMonitor, Database, DownloadOptions, DownloadProgress,
    DownloadStatus, Downloader, QueueEvent, QueueManager, TokenBucketRateLimiter,
};

#[derive(Parser, Debug)]
#[command(
    name = "cosmodownloader-cli",
    author = "Mahedi",
    version,
    about = "High-performance multi-segment download manager for Windows 11",
    long_about = "CosmoDownloader is an open-source, multi-segment, resume-capable download manager with SQLite queue persistence and speed limiting."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Direct URL to download immediately (shorthand: cosmodownloader <URL>)
    #[arg(index = 1)]
    url: Option<String>,

    /// Destination file or directory path
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Number of parallel segments (1 to 32, default: 8)
    #[arg(short = 'n', long = "connections", default_value_t = 8)]
    connections: usize,

    /// Speed limit in bytes/sec (0 = unlimited)
    #[arg(short = 's', long = "speed")]
    speed_limit: Option<u64>,

    /// Custom HTTP header in 'Key: Value' format
    #[arg(short = 'H', long = "header")]
    headers: Vec<String>,

    /// Custom User-Agent header
    #[arg(short = 'A', long = "user-agent")]
    user_agent: Option<String>,

    /// Custom HTTP Referer header
    #[arg(short = 'e', long = "referer")]
    referer: Option<String>,

    /// Custom HTTP Cookie header
    #[arg(short = 'b', long = "cookie")]
    cookie: Option<String>,

    /// Expected SHA-256 checksum to verify after completion
    #[arg(short = 'c', long = "checksum")]
    checksum: Option<String>,

    /// Disable auto-resuming from existing partial files
    #[arg(long = "no-resume")]
    no_resume: bool,

    /// Enable verbose diagnostic logging
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Download a file directly with interactive progress
    Download {
        url: String,
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
        #[arg(short = 'n', long = "connections", default_value_t = 8)]
        connections: usize,
        #[arg(short = 's', long = "speed")]
        speed_limit: Option<u64>,
        #[arg(short = 'H', long = "header")]
        headers: Vec<String>,
        #[arg(short = 'A', long = "user-agent")]
        user_agent: Option<String>,
        #[arg(short = 'e', long = "referer")]
        referer: Option<String>,
        #[arg(short = 'b', long = "cookie")]
        cookie: Option<String>,
        #[arg(short = 'c', long = "checksum")]
        checksum: Option<String>,
        #[arg(long = "no-resume")]
        no_resume: bool,
    },
    /// Add a download task to the persistent SQLite queue
    Add {
        url: String,
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
        #[arg(short = 'C', long = "category")]
        category: Option<String>,
        #[arg(short = 'n', long = "connections", default_value_t = 8)]
        connections: usize,
        #[arg(short = 's', long = "speed", default_value_t = 0)]
        speed_limit: u64,
        #[arg(short = 'c', long = "checksum")]
        checksum: Option<String>,
    },
    /// List all downloads from the SQLite database
    List {
        #[arg(short = 's', long = "status")]
        status: Option<String>,
    },
    /// Pause an active or queued download
    Pause {
        id: String,
    },
    /// Resume a paused download
    Resume {
        id: String,
    },
    /// Remove a download task from the database
    Remove {
        id: String,
        #[arg(short = 'd', long = "delete-file")]
        delete_file: bool,
    },
    /// Run the queue processing daemon
    Queue {
        #[arg(short = 'm', long = "max-concurrent", default_value_t = 3)]
        max_concurrent: usize,
        #[arg(short = 's', long = "global-speed", default_value_t = 0)]
        global_speed: u64,
    },
    /// Monitor the system clipboard for downloadable URLs
    Monitor,
}

struct DirectDownloadArgs {
    url: String,
    output: Option<PathBuf>,
    connections: usize,
    speed_limit: Option<u64>,
    headers: Vec<String>,
    user_agent: Option<String>,
    referer: Option<String>,
    cookie: Option<String>,
    checksum: Option<String>,
    no_resume: bool,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        EnvFilter::new("rustget_core=debug,rustget_cli=debug")
    } else {
        EnvFilter::new("rustget_core=info,rustget_cli=info")
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    match cli.command {
        Some(Commands::Download {
            url,
            output,
            connections,
            speed_limit,
            headers,
            user_agent,
            referer,
            cookie,
            checksum,
            no_resume,
        }) => {
            let args = DirectDownloadArgs {
                url, output, connections, speed_limit, headers, user_agent, referer, cookie, checksum, no_resume,
            };
            run_direct_download(args).await
        }
        Some(Commands::Add {
            url,
            output,
            category,
            connections,
            speed_limit,
            checksum,
        }) => {
            let db = Database::open_default()?;
            let (event_tx, _) = mpsc::channel(32);
            let queue = QueueManager::new(db, 3, 0, event_tx);

            let cat = category.and_then(|c| Category::from_name(&c));
            let options = DownloadOptions {
                url,
                output_path: output,
                num_segments: connections,
                expected_sha256: checksum,
                ..Default::default()
            };

            let id = queue.add_download(options, cat, speed_limit).await?;
            println!("✓ Enqueued download task successfully!");
            println!("  Task ID: {}", id);
            println!("  Run 'cosmodownloader-cli queue' to start processing the queue.");
            Ok(())
        }
        Some(Commands::List { status }) => {
            let db = Database::open_default()?;
            let downloads = db.list_downloads(status.as_deref()).await?;

            if downloads.is_empty() {
                println!("No downloads found.");
                return Ok(());
            }

            println!("{:<36} | {:<22} | {:<12} | {:<10} | {:<12}", "ID", "FILENAME", "STATUS", "CATEGORY", "PROGRESS");
            println!("{:-<36}-+-{:-<22}-+-{:-<12}-+-{:-<10}-+-{:-<12}", "", "", "", "", "");

            for d in downloads {
                let prog_str = match d.total_size {
                    Some(total) if total > 0 => {
                        format!("{:.1}% ({})", (d.downloaded_bytes as f64 / total as f64) * 100.0, SpeedTracker::format_bytes(d.downloaded_bytes))
                    }
                    _ => SpeedTracker::format_bytes(d.downloaded_bytes),
                };

                let short_name = if d.filename.len() > 20 {
                    format!("{}...", &d.filename[..17])
                } else {
                    d.filename.clone()
                };

                println!(
                    "{:<36} | {:<22} | {:<12} | {:<10} | {:<12}",
                    d.id, short_name, d.status.to_uppercase(), d.category, prog_str
                );
            }
            Ok(())
        }
        Some(Commands::Pause { id }) => {
            let db = Database::open_default()?;
            let (event_tx, _) = mpsc::channel(32);
            let queue = QueueManager::new(db, 3, 0, event_tx);
            queue.pause(&id).await?;
            println!("✓ Paused download task: {}", id);
            Ok(())
        }
        Some(Commands::Resume { id }) => {
            let db = Database::open_default()?;
            let (event_tx, _) = mpsc::channel(32);
            let queue = QueueManager::new(db, 3, 0, event_tx);
            queue.resume(&id).await?;
            println!("✓ Resumed download task: {}", id);
            Ok(())
        }
        Some(Commands::Remove { id, delete_file }) => {
            let db = Database::open_default()?;
            let (event_tx, _) = mpsc::channel(32);
            let queue = QueueManager::new(db, 3, 0, event_tx);
            queue.remove(&id, delete_file).await?;
            println!("✓ Removed download task: {}", id);
            Ok(())
        }
        Some(Commands::Queue { max_concurrent, global_speed }) => {
            let db = Database::open_default()?;
            let (event_tx, mut event_rx) = mpsc::channel(64);
            let queue = Arc::new(QueueManager::new(db, max_concurrent, global_speed, event_tx));

            let cancel = CancellationToken::new();
            let c_ctrlc = cancel.clone();
            tokio::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    println!("\nShutting down download queue manager...");
                    c_ctrlc.cancel();
                }
            });

            println!("============================================================");
            println!(" Cosmo Downloader Queue Daemon Active");
            println!(" Max Concurrent Downloads: {}", max_concurrent);
            if global_speed > 0 {
                println!(" Global Speed Limit: {}", SpeedTracker::format_speed(global_speed as f64));
            } else {
                println!(" Global Speed Limit: Unlimited");
            }
            println!(" Press Ctrl+C to stop.");
            println!("============================================================");

            let q_clone = Arc::clone(&queue);
            let q_cancel = cancel.clone();
            tokio::spawn(async move {
                q_clone.run_queue_worker(q_cancel).await;
            });

            while let Some(event) = event_rx.recv().await {
                match event {
                    QueueEvent::TaskAdded(id) => println!("[+] Task added: {}", id),
                    QueueEvent::StatusChanged { id, status } => println!("[*] Task {} is now {}", &id[..8], status),
                    QueueEvent::TaskCompleted(id) => println!("[✓] Task {} finished successfully!", &id[..8]),
                    QueueEvent::TaskFailed { id, error } => println!("[✗] Task {} failed: {}", &id[..8], error),
                    QueueEvent::Progress { .. } => {}
                }
            }

            Ok(())
        }
        Some(Commands::Monitor) => {
            let cancel = CancellationToken::new();
            let c_ctrlc = cancel.clone();
            tokio::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    println!("\nStopping clipboard monitor...");
                    c_ctrlc.cancel();
                }
            });

            println!("============================================================");
            println!(" Cosmo Downloader Clipboard Monitor Active");
            println!(" Copy any download link to test. Press Ctrl+C to exit.");
            println!("============================================================");

            let monitor = ClipboardMonitor::new(cancel.clone());
            let (url_tx, mut url_rx) = mpsc::channel(16);
            monitor.start(url_tx);

            let db = Database::open_default()?;
            let (q_tx, _) = mpsc::channel(32);
            let queue = QueueManager::new(db, 3, 0, q_tx);

            while let Some(url) = url_rx.recv().await {
                println!("\n[!] Clipboard detected download URL: {}", url);
                let opt = DownloadOptions {
                    url: url.clone(),
                    ..Default::default()
                };
                match queue.add_download(opt, None, 0).await {
                    Ok(id) => println!("    ✓ Automatically queued task: {}", id),
                    Err(e) => eprintln!("    ✗ Failed to queue URL: {}", e),
                }
            }

            Ok(())
        }
        None => {
            if let Some(url) = cli.url {
                let args = DirectDownloadArgs {
                    url,
                    output: cli.output,
                    connections: cli.connections,
                    speed_limit: cli.speed_limit,
                    headers: cli.headers,
                    user_agent: cli.user_agent,
                    referer: cli.referer,
                    cookie: cli.cookie,
                    checksum: cli.checksum,
                    no_resume: cli.no_resume,
                };
                run_direct_download(args).await
            } else {
                bail!("No command or URL specified. Run 'cosmodownloader-cli --help' for usage.");
            }
        }
    }
}

async fn run_direct_download(args: DirectDownloadArgs) -> Result<()> {
    let mut header_map = HeaderMap::new();
    for h in &args.headers {
        if let Some((k, v)) = h.split_once(':') {
            let name = HeaderName::from_bytes(k.trim().as_bytes())
                .with_context(|| format!("Invalid header name: '{}'", k))?;
            let val = HeaderValue::from_str(v.trim())
                .with_context(|| format!("Invalid header value: '{}'", v))?;
            header_map.insert(name, val);
        }
    }

    let cancel_token = CancellationToken::new();
    let ctrlc_token = cancel_token.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            println!("\nReceived Ctrl+C interrupt. Pausing download and saving state to disk...");
            ctrlc_token.cancel();
        }
    });

    let options = DownloadOptions {
        url: args.url.clone(),
        output_path: args.output,
        num_segments: args.connections,
        headers: header_map,
        user_agent: args.user_agent,
        referer: args.referer,
        cookie: args.cookie,
        expected_sha256: args.checksum,
        resume: !args.no_resume,
    };

    println!("============================================================");
    println!(" Cosmo Downloader v{} - High Performance Download Engine", env!("CARGO_PKG_VERSION"));
    println!(" URL: {}", args.url);
    println!(" Segments: {}", args.connections.clamp(1, 32));
    if let Some(spd) = args.speed_limit {
        println!(" Speed Limit: {}", SpeedTracker::format_speed(spd as f64));
    }
    println!("============================================================");

    let mut downloader = Downloader::new(options, cancel_token.clone());
    if let Some(spd) = args.speed_limit {
        downloader = downloader.with_rate_limiter(TokenBucketRateLimiter::new(spd));
    }

    let (progress_tx, mut progress_rx) = mpsc::channel::<DownloadProgress>(64);
    let download_task = tokio::spawn(async move {
        downloader.start(progress_tx).await
    });

    let mut last_percent = -1.0;
    while let Some(prog) = progress_rx.recv().await {
        render_progress(&prog, &mut last_percent);
    }

    match download_task.await? {
        Ok(path) => {
            println!("\n\n Download completed successfully!");
            println!(" Saved to: {}", path.display());
            Ok(())
        }
        Err(rustget_core::DownloadError::Cancelled) => {
            println!("\n Download paused. You can resume anytime by re-running the exact same command.");
            Ok(())
        }
        Err(e) => {
            eprintln!("\n\n Download failed: {}", e);
            Err(e.into())
        }
    }
}

fn render_progress(prog: &DownloadProgress, last_percent: &mut f64) {
    match &prog.status {
        DownloadStatus::Probing => {
            print!("\r[~] Probing remote server and checking range support...               ");
            let _ = stdout().flush();
        }
        DownloadStatus::Allocating => {
            print!("\r[~] Pre-allocating disk space for {} segments...                       ", prog.segments.len());
            let _ = stdout().flush();
        }
        DownloadStatus::Downloading => {
            let pct = prog.percent();
            *last_percent = pct;

            let total_str = match prog.total_bytes {
                Some(total) => SpeedTracker::format_bytes(total),
                None => "Unknown".to_string(),
            };
            let dl_str = SpeedTracker::format_bytes(prog.downloaded_bytes);
            let speed_str = SpeedTracker::format_speed(prog.speed_bps);
            let eta_str = SpeedTracker::format_eta(prog.eta_seconds);

            let bar_width: usize = 25;
            let filled = ((pct / 100.0) * (bar_width as f64)).round() as usize;
            let empty = bar_width.saturating_sub(filled);
            let bar: String = "█".repeat(filled) + &"░".repeat(empty);

            print!(
                "\r[{}] {:>5.1}% | {:>9} / {:<9} | {:>10} | ETA: {:<8}",
                bar, pct, dl_str, total_str, speed_str, eta_str
            );

            if prog.segments.len() > 1 && prog.segments.len() <= 8 {
                let seg_strs: Vec<String> = prog
                    .segments
                    .iter()
                    .map(|s| {
                        if s.complete {
                            format!("{}:100%", s.index + 1)
                        } else {
                            format!("{}:{:>2.0}%", s.index + 1, s.progress_percent())
                        }
                    })
                    .collect();
                print!(" | [{}]", seg_strs.join(" "));
            }

            let _ = stdout().flush();
        }
        DownloadStatus::Verifying => {
            print!("\r[~] Verifying SHA-256 checksum against expected hash...               ");
            let _ = stdout().flush();
        }
        DownloadStatus::Paused => {
            print!("\r[||] Paused                                                            ");
            let _ = stdout().flush();
        }
        DownloadStatus::Completed => {
            print!("\r[✓] 100.0% Complete                                                    ");
            let _ = stdout().flush();
        }
        DownloadStatus::Failed(msg) => {
            print!("\r[✗] Error: {}                                                          ", msg);
            let _ = stdout().flush();
        }
    }
}
