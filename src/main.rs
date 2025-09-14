use anyhow::Result;
use clap::{Parser, Subcommand};
use gitup_core::{Repository, FileStatus, ConflictResolution};
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
        /// Remote name
        #[arg(default_value = "origin")]
        remote: String,
        /// Path to the repository
        #[arg(short = 'p', long, default_value = ".")]
        path: PathBuf,
    },
    /// Pull from remote
    Pull {
        /// Remote name
        #[arg(default_value = "origin")]
        remote: String,
        /// Branch name (defaults to current branch)
        branch: Option<String>,
        /// Path to the repository
        #[arg(short = 'p', long, default_value = ".")]
        path: PathBuf,
    },
    /// Push to remote
    Push {
        /// Remote name
        #[arg(default_value = "origin")]
        remote: String,
        /// Path to the repository
        #[arg(short = 'p', long, default_value = ".")]
        path: PathBuf,
        /// Set upstream
        #[arg(short = 'u', long)]
        set_upstream: bool,
    },
    /// Manage stashes
    Stash {
        /// Path to the repository
        #[arg(short = 'p', long, default_value = ".")]
        path: PathBuf,
        #[command(subcommand)]
        command: StashCommands,
    },
    /// Manage tags
    Tag {
        /// Path to the repository
        #[arg(short = 'p', long, default_value = ".")]
        path: PathBuf,
        #[command(subcommand)]
        command: TagCommands,
    },
    /// Merge branches
    Merge {
        /// Path to the repository
        #[arg(short = 'p', long, default_value = ".")]
        path: PathBuf,
        #[command(subcommand)]
        command: MergeCommands,
    },
}

#[derive(Subcommand)]
enum StashCommands {
    /// Save changes to stash
    Save {
        /// Stash message
        #[arg(short, long)]
        message: Option<String>,
        /// Include untracked files
        #[arg(short = 'u', long)]
        include_untracked: bool,
    },
    /// List all stashes
    List,
    /// Apply a stash
    Apply {
        /// Stash index (default: 0)
        #[arg(default_value = "0")]
        index: usize,
    },
    /// Pop a stash (apply and remove)
    Pop {
        /// Stash index (default: 0)
        index: Option<usize>,
    },
    /// Drop a stash
    Drop {
        /// Stash index
        index: usize,
    },
    /// Show a stash
    Show {
        /// Stash index (default: 0)
        #[arg(default_value = "0")]
        index: usize,
    },
    /// Clear all stashes
    Clear,
}

#[derive(Subcommand)]
enum TagCommands {
    /// Create a new tag
    Create {
        /// Tag name
        name: String,
        /// Target commit (default: HEAD)
        #[arg(short, long)]
        target: Option<String>,
        /// Tag message (creates annotated tag)
        #[arg(short, long)]
        message: Option<String>,
        /// Force create tag even if it exists
        #[arg(short, long)]
        force: bool,
    },
    /// List tags
    List {
        /// Pattern to filter tags
        pattern: Option<String>,
    },
    /// Delete a tag
    Delete {
        /// Tag name
        name: String,
    },
    /// Show tag details
    Show {
        /// Tag name
        name: String,
    },
    /// Push tags to remote
    Push {
        /// Remote name
        #[arg(default_value = "origin")]
        remote: String,
        /// Tag name (push all tags if not specified)
        tag: Option<String>,
        /// Force push
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum MergeCommands {
    /// Merge a branch into the current branch
    Branch {
        /// Branch name to merge
        name: String,
        /// Custom merge commit message
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Abort an in-progress merge
    Abort,
    /// Continue an in-progress merge after resolving conflicts
    Continue {
        /// Custom merge commit message
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Show merge status
    Status,
    /// List conflicted files
    Conflicts,
    /// Resolve a conflict by choosing a version
    Resolve {
        /// File path to resolve
        file: String,
        /// Resolution strategy (ours, theirs, manual)
        #[arg(value_enum)]
        strategy: ResolutionStrategy,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum ResolutionStrategy {
    Ours,
    Theirs,
    Manual,
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
        Commands::Fetch { remote, path } => {
            let repo = Repository::open(&path)?;
            println!("Fetching from {}...", remote);
            let result = repo.fetch(&remote)?;
            println!("{}", result);
        }
        Commands::Pull { remote, branch, path } => {
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
        Commands::Push { remote, path, set_upstream } => {
            let repo = Repository::open(&path)?;

            println!("Pushing to {}...", remote);
            let result = repo.push(&remote)?;
            println!("{}", result);

            if set_upstream {
                // Get current branch name
                let branches = repo.list_branches()?;
                let current_branch = branches.iter()
                    .find(|b| b.is_head && !b.is_remote)
                    .map(|b| b.name.clone())
                    .ok_or_else(|| anyhow::anyhow!("No current branch"))?;

                // Try to set upstream after push
                match repo.set_upstream(&remote, &current_branch) {
                    Ok(_) => println!("Branch '{}' set to track '{}/{}'", current_branch, remote, current_branch),
                    Err(e) => eprintln!("Note: Could not set upstream: {}", e),
                }
            }
        }
        Commands::Stash { path, command } => {
            let repo = Repository::open(&path)?;

            match command {
                StashCommands::Save { message, include_untracked } => {
                    let result = repo.stash_save(message.as_deref(), include_untracked)?;
                    println!("{}", result);
                }
                StashCommands::List => {
                    let stashes = repo.stash_list()?;
                    if stashes.is_empty() {
                        println!("No stashes found");
                    } else {
                        for stash in stashes {
                            println!("stash@{{{}}}: {}", stash.index, stash.message);
                        }
                    }
                }
                StashCommands::Apply { index } => {
                    let result = repo.stash_apply(index)?;
                    println!("{}", result);
                }
                StashCommands::Pop { index } => {
                    let result = repo.stash_pop(index)?;
                    println!("{}", result);
                }
                StashCommands::Drop { index } => {
                    let result = repo.stash_drop(index)?;
                    println!("{}", result);
                }
                StashCommands::Show { index } => {
                    let result = repo.stash_show(index)?;
                    println!("{}", result);
                }
                StashCommands::Clear => {
                    let result = repo.stash_clear()?;
                    println!("{}", result);
                }
            }
        }
        Commands::Tag { path, command } => {
            let repo = Repository::open(&path)?;

            match command {
                TagCommands::Create { name, target, message, force } => {
                    let result = repo.tag_create(&name, target.as_deref(), message.as_deref(), force)?;
                    println!("{}", result);
                }
                TagCommands::List { pattern } => {
                    let tags = repo.tag_list(pattern.as_deref())?;
                    if tags.is_empty() {
                        println!("No tags found");
                    } else {
                        for tag in tags {
                            if tag.is_annotated {
                                println!("{} (annotated)", tag.name);
                            } else {
                                println!("{}", tag.name);
                            }
                        }
                    }
                }
                TagCommands::Delete { name } => {
                    let result = repo.tag_delete(&name)?;
                    println!("{}", result);
                }
                TagCommands::Show { name } => {
                    let result = repo.tag_show(&name)?;
                    print!("{}", result);
                }
                TagCommands::Push { remote, tag, force } => {
                    let result = repo.tag_push(&remote, tag.as_deref(), force)?;
                    println!("{}", result);
                }
            }
        }
        Commands::Merge { path, command } => {
            let repo = Repository::open(&path)?;

            match command {
                MergeCommands::Branch { name, message } => {
                    println!("Merging branch '{}'...", name);
                    let result = repo.merge_branch(&name, message.as_deref())?;

                    if result.success {
                        println!("{}", result.message);
                        if let Some(commit) = result.merged_commit {
                            println!("Created merge commit: {}", &commit[..8]);
                        }
                    } else {
                        println!("CONFLICT: {}", result.message);
                        if !result.conflicts.is_empty() {
                            println!("\nConflicted files:");
                            for conflict in &result.conflicts {
                                println!("  - {}", conflict);
                            }
                            println!("\nResolve conflicts, stage changes, and run 'gitup merge continue'");
                        }
                    }
                }
                MergeCommands::Abort => {
                    let result = repo.merge_abort()?;
                    println!("{}", result);
                }
                MergeCommands::Continue { message } => {
                    let result = repo.merge_continue(message.as_deref())?;

                    if result.success {
                        println!("{}", result.message);
                        if let Some(commit) = result.merged_commit {
                            println!("Created merge commit: {}", &commit[..8]);
                        }
                    } else {
                        println!("Cannot continue: {}", result.message);
                        if !result.conflicts.is_empty() {
                            println!("\nConflicted files:");
                            for conflict in &result.conflicts {
                                println!("  - {}", conflict);
                            }
                        }
                    }
                }
                MergeCommands::Status => {
                    let status = repo.merge_status()?;
                    println!("{}", status);
                }
                MergeCommands::Conflicts => {
                    let conflicts = repo.merge_conflicts()?;
                    if conflicts.is_empty() {
                        println!("No conflicts");
                    } else {
                        println!("Conflicted files:");
                        for conflict in conflicts {
                            println!("  - {}", conflict);
                        }
                    }
                }
                MergeCommands::Resolve { file, strategy } => {
                    let resolution = match strategy {
                        ResolutionStrategy::Ours => ConflictResolution::Ours,
                        ResolutionStrategy::Theirs => ConflictResolution::Theirs,
                        ResolutionStrategy::Manual => ConflictResolution::Manual,
                    };
                    let result = repo.merge_resolve_conflict(&file, resolution)?;
                    println!("{}", result);
                }
            }
        }
    }

    Ok(())
}