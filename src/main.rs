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
    /// Stage files
    Stage {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Files to stage (empty for all)
        files: Vec<String>,
        /// Stage all files
        #[arg(short, long)]
        all: bool,
    },
    /// Unstage files
    Unstage {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Files to unstage (empty for all)
        files: Vec<String>,
        /// Unstage all files
        #[arg(short, long)]
        all: bool,
    },
    /// Create a commit
    Commit {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Commit message
        #[arg(short, long)]
        message: String,
        /// Author name
        #[arg(long)]
        author: Option<String>,
        /// Author email
        #[arg(long)]
        email: Option<String>,
        /// Amend the last commit
        #[arg(long)]
        amend: bool,
    },
    /// Launch Terminal UI
    Tui {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Manage remotes
    Remote {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
        #[command(subcommand)]
        command: RemoteCommands,
    },
    /// Fetch from remote
    Fetch {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Remote name
        #[arg(default_value = "origin")]
        remote: String,
    },
    /// Pull from remote
    Pull {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Remote name
        #[arg(default_value = "origin")]
        remote: String,
        /// Branch name (defaults to current branch)
        branch: Option<String>,
    },
    /// Push to remote
    Push {
        /// Path to the repository
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Remote name
        #[arg(default_value = "origin")]
        remote: String,
        /// Set upstream
        #[arg(short = 'u', long)]
        set_upstream: bool,
    },
}

#[derive(Subcommand)]
enum RemoteCommands {
    /// List remotes
    List,
    /// Add a remote
    Add {
        /// Remote name
        name: String,
        /// Remote URL
        url: String,
    },
    /// Remove a remote
    Remove {
        /// Remote name
        name: String,
    },
    /// Show remote info
    Show {
        /// Remote name
        name: String,
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
        Commands::Stage { path, files, all } => {
            let repo = Repository::open(&path)?;

            if all || files.is_empty() {
                repo.stage_all()?;
                println!("Staged all changes");
            } else {
                for file in files {
                    repo.stage_file(&file)?;
                    println!("Staged: {}", file);
                }
            }
        }
        Commands::Unstage { path, files, all } => {
            let repo = Repository::open(&path)?;

            if all || files.is_empty() {
                repo.reset_index()?;
                println!("Unstaged all changes");
            } else {
                for file in files {
                    repo.unstage_file(&file)?;
                    println!("Unstaged: {}", file);
                }
            }
        }
        Commands::Commit { path, message, author, email, amend } => {
            let repo = Repository::open(&path)?;

            // Check if there are changes to commit
            if !amend && !repo.has_staged_changes()? {
                println!("No changes staged for commit");
                return Ok(());
            }

            let commit_id = if amend {
                repo.amend_commit(Some(&message))?
            } else {
                // Get author info from git config or use defaults
                let config = git2::Config::open_default().ok();
                let author_name = author.or_else(|| {
                    config.as_ref().and_then(|c| c.get_string("user.name").ok())
                }).unwrap_or_else(|| "GitUp User".to_string());

                let author_email = email.or_else(|| {
                    config.as_ref().and_then(|c| c.get_string("user.email").ok())
                }).unwrap_or_else(|| "gitup@local".to_string());

                repo.commit(&message, &author_name, &author_email)?
            };

            println!("Created commit: {}", &commit_id[..8]);
        }
        Commands::Tui { path } => {
            gitup_ui::run_tui(&path)?;
        }
        Commands::Remote { path, command } => {
            let repo = Repository::open(&path)?;

            match command {
                RemoteCommands::List => {
                    let remotes = repo.list_remotes()?;
                    if remotes.is_empty() {
                        println!("No remotes configured");
                    } else {
                        for remote in remotes {
                            println!("{}\t{} (fetch)", remote.name, remote.url);
                            if let Some(push_url) = remote.push_url {
                                if push_url != remote.url {
                                    println!("{}\t{} (push)", remote.name, push_url);
                                }
                            }
                        }
                    }
                }
                RemoteCommands::Add { name, url } => {
                    repo.add_remote(&name, &url)?;
                    println!("Added remote {} -> {}", name, url);
                }
                RemoteCommands::Remove { name } => {
                    repo.remove_remote(&name)?;
                    println!("Removed remote {}", name);
                }
                RemoteCommands::Show { name } => {
                    let remotes = repo.list_remotes()?;
                    if let Some(remote) = remotes.iter().find(|r| r.name == name) {
                        println!("* remote {}", remote.name);
                        println!("  Fetch URL: {}", remote.url);
                        if let Some(push_url) = &remote.push_url {
                            println!("  Push URL: {}", push_url);
                        }
                    } else {
                        println!("Remote '{}' not found", name);
                    }
                }
            }
        }
        Commands::Fetch { path, remote } => {
            let repo = Repository::open(&path)?;
            println!("Fetching from {}...", remote);
            let result = repo.fetch(&remote)?;
            println!("{}", result);
        }
        Commands::Pull { path, remote, branch } => {
            let repo = Repository::open(&path)?;

            // Get current branch if not specified
            let branch_name = if let Some(b) = branch {
                b
            } else {
                // Get current branch name
                let branches = repo.list_branches()?;
                branches.iter()
                    .find(|b| b.is_head && !b.is_remote)
                    .map(|b| b.name.clone())
                    .ok_or_else(|| anyhow::anyhow!("No current branch"))?
            };

            println!("Pulling from {} {}", remote, branch_name);
            let result = repo.pull(&remote, &branch_name)?;
            println!("{}", result);
        }
        Commands::Push { path, remote, set_upstream } => {
            let repo = Repository::open(&path)?;

            if set_upstream {
                // Get current branch name
                let branches = repo.list_branches()?;
                let current_branch = branches.iter()
                    .find(|b| b.is_head && !b.is_remote)
                    .map(|b| b.name.clone())
                    .ok_or_else(|| anyhow::anyhow!("No current branch"))?;

                repo.set_upstream(&remote, &current_branch)?;
                println!("Set upstream to {}/{}", remote, current_branch);
            }

            println!("Pushing to {}...", remote);
            let result = repo.push(&remote)?;
            println!("{}", result);
        }
    }

    Ok(())
}