use anyhow::Result;
use clap::{Parser, Subcommand};
use gitup_core::{Repository, FileStatus};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gitup")]
#[command(about = "A fast Git client written in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Open a repository
    Open {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Initialize a new repository
    Init {
        /// Path for the new repository
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Show repository status
    Status {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// List branches
    Branches {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Show recent commits
    Log {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Number of commits to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    /// Show diff
    Diff {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Show staged changes
        #[arg(long)]
        staged: bool,
        /// Show diff for a specific commit
        #[arg(long)]
        commit: Option<String>,
        /// Show diff between two commits
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        /// Show statistics only
        #[arg(long)]
        stat: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Open { path } => {
            let repo = Repository::open(&path)?;
            println!("Opened repository at: {}", path.display());
            if repo.is_clean()? {
                println!("Repository is clean");
            } else {
                println!("Repository has uncommitted changes");
            }
        }
        Commands::Init { path } => {
            Repository::init(&path)?;
            println!("Initialized empty repository at: {}", path.display());
        }
        Commands::Status { path } => {
            let repo = Repository::open(&path)?;
            if repo.is_clean()? {
                println!("Working tree clean");
            } else {
                println!("Working tree has uncommitted changes");
            }
        }
        Commands::Branches { path } => {
            let repo = Repository::open(&path)?;
            let branches = repo.list_branches()?;

            println!("Local branches:");
            for branch in branches.iter().filter(|b| !b.is_remote) {
                let marker = if branch.is_head { "* " } else { "  " };
                println!("{}{}", marker, branch.name);
            }

            println!("\nRemote branches:");
            for branch in branches.iter().filter(|b| b.is_remote) {
                println!("  {}", branch.name);
            }
        }
        Commands::Log { path, count } => {
            let repo = Repository::open(&path)?;
            let commits = repo.get_commits(count)?;

            for commit in commits {
                println!("commit {}", &commit.id[..8]);
                println!("Author: {} <{}>", commit.author, commit.email);
                println!("Date:   {}", chrono::DateTime::from_timestamp(commit.timestamp, 0)
                    .map(|dt| dt.to_string())
                    .unwrap_or_default());
                println!("\n    {}\n", commit.message);
            }
        }
        Commands::Diff { path, staged, commit, from, to, stat } => {
            let repo = Repository::open(&path)?;

            let diffs = if let Some(commit_id) = commit {
                repo.diff_for_commit(&commit_id)?
            } else if let (Some(old), Some(new)) = (from, to) {
                repo.diff_between_commits(&old, &new)?
            } else if staged {
                repo.diff_index_to_head()?
            } else {
                repo.diff_workdir_to_index()?
            };

            if stat {
                // Show statistics only
                let stats = gitup_core::DiffStats::from_diffs(&diffs);
                println!(" {} files changed, {} insertions(+), {} deletions(-)",
                    stats.files_changed, stats.insertions, stats.deletions);
            } else {
                // Show full diff
                for file_diff in diffs {
                    let status_char = match file_diff.file.status {
                        FileStatus::Added => "+",
                        FileStatus::Deleted => "-",
                        FileStatus::Modified => "M",
                        FileStatus::Renamed => "R",
                        _ => "?",
                    };

                    println!("{} {}", status_char, file_diff.file.path);

                    if !file_diff.binary {
                        for line in &file_diff.lines {
                            let prefix = match line.origin {
                                gitup_core::LineOrigin::Addition => "+",
                                gitup_core::LineOrigin::Deletion => "-",
                                gitup_core::LineOrigin::Context => " ",
                            };
                            print!("{}{}", prefix, line.content);
                            if !line.content.ends_with('\n') {
                                println!();
                            }
                        }
                    } else {
                        println!("Binary file");
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}