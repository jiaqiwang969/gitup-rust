use git2::{Repository, Sort, Commit, Oid, Signature};
use chrono::{Utc, TimeZone};
use anyhow::{Result, Context};
use crate::core::{Dag, CommitNode};

pub struct GitWalker {
    repo: Repository,
}

impl GitWalker {
    pub fn new(repo_path: Option<&str>) -> Result<Self> {
        let repo = match repo_path {
            Some(path) => Repository::open(path),
            None => Repository::open_from_env(),
        }.context("Failed to open repository")?;

        Ok(Self { repo })
    }

    /// Convert git repository commits to DAG
    pub fn into_dag(&self, limit: Option<usize>) -> Result<Dag> {
        let mut dag = Dag::new();
        let mut revwalk = self.repo.revwalk()?;

        // Start from HEAD and all branches
        revwalk.push_head()?;
        for branch in self.repo.branches(None)? {
            let (branch, _) = branch?;
            if let Some(target) = branch.get().target() {
                revwalk.push(target)?;
            }
        }

        // Sort by topological order and time
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

        let mut count = 0;
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;

            let node = self.commit_to_node(&commit)?;
            dag.add_node(node);

            count += 1;
            if let Some(limit) = limit {
                if count >= limit {
                    break;
                }
            }
        }

        Ok(dag)
    }

    /// Convert a git2::Commit to CommitNode
    fn commit_to_node(&self, commit: &Commit) -> Result<CommitNode> {
        let id = commit.id().to_string();
        let parents: Vec<String> = commit.parent_ids().map(|oid| oid.to_string()).collect();

        // Convert git time to DateTime<Utc>
        let timestamp = Utc.timestamp_opt(commit.time().seconds(), 0)
            .single()
            .context("Invalid commit timestamp")?;

        let author = commit.author().name()
            .unwrap_or("Unknown")
            .to_string();

        let message = commit.summary()
            .unwrap_or("")
            .to_string();

        Ok(CommitNode::new(id, parents, timestamp, author, message))
    }

    /// Get branch references
    pub fn get_refs(&self) -> Result<Vec<(String, String)>> {
        let mut refs = Vec::new();

        // Get branches
        for branch in self.repo.branches(None)? {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                if let Some(target) = branch.get().target() {
                    refs.push((name.to_string(), target.to_string()));
                }
            }
        }

        // Get tags
        self.repo.tag_foreach(|oid, name| {
            if let Ok(name_str) = std::str::from_utf8(name) {
                refs.push((name_str.to_string(), oid.to_string()));
            }
            true
        })?;

        Ok(refs)
    }

    /// Get HEAD reference
    pub fn get_head(&self) -> Result<Option<String>> {
        match self.repo.head() {
            Ok(head) => {
                Ok(head.target().map(|oid| oid.to_string()))
            }
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> Result<(TempDir, Repository)> {
        let dir = TempDir::new()?;
        let repo = Repository::init(dir.path())?;

        // Configure repo
        let mut config = repo.config()?;
        config.set_str("user.name", "Test User")?;
        config.set_str("user.email", "test@example.com")?;

        Ok((dir, repo))
    }

    fn commit_to_repo(repo: &Repository, message: &str, parents: &[&Commit], update_ref: Option<&str>) -> Result<Oid> {
        let sig = Signature::now("Test User", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;

        Ok(repo.commit(
            update_ref,
            &sig,
            &sig,
            message,
            &tree,
            parents,
        )?)
    }

    #[test]
    fn test_single_commit_dag() -> Result<()> {
        let (_dir, repo) = create_test_repo()?;

        // Create a single commit
        let sig = Signature::now("Test User", "test@example.com")?;
        let tree_id = {
            let mut index = repo.index()?;
            index.write_tree()?
        };
        let tree = repo.find_tree(tree_id)?;
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

        let walker = GitWalker::new(Some(repo.path().to_str().unwrap()))?;
        let dag = walker.into_dag(None)?;

        assert_eq!(dag.node_count(), 1);
        assert_eq!(dag.edge_count(), 0);
        assert_eq!(dag.roots().len(), 1);

        Ok(())
    }

    #[test]
    fn test_linear_history() -> Result<()> {
        let (_dir, repo) = create_test_repo()?;

        let oid1 = commit_to_repo(&repo, "First commit", &[], Some("HEAD"))?;
        let commit1 = repo.find_commit(oid1)?;

        let oid2 = commit_to_repo(&repo, "Second commit", &[&commit1], Some("HEAD"))?;
        let commit2 = repo.find_commit(oid2)?;

        let _oid3 = commit_to_repo(&repo, "Third commit", &[&commit2], Some("HEAD"))?;

        let walker = GitWalker::new(Some(repo.path().to_str().unwrap()))?;
        let dag = walker.into_dag(None)?;

        assert_eq!(dag.node_count(), 3);
        assert_eq!(dag.edge_count(), 2);
        assert_eq!(dag.roots().len(), 1);

        Ok(())
    }

    #[test]
    fn test_merge_commit() -> Result<()> {
        let (_dir, repo) = create_test_repo()?;

        // Create base commit
        let base_oid = commit_to_repo(&repo, "Base commit", &[], Some("HEAD"))?;
        let base_commit = repo.find_commit(base_oid)?;

        // Create branch 1
        let branch1_oid = commit_to_repo(&repo, "Branch 1", &[&base_commit], Some("HEAD"))?;
        let branch1_commit = repo.find_commit(branch1_oid)?;

        // Create branch 2 (from base, not HEAD)
        let branch2_oid = commit_to_repo(&repo, "Branch 2", &[&base_commit], None)?;
        let branch2_commit = repo.find_commit(branch2_oid)?;

        // Create merge commit
        let _merge_oid = commit_to_repo(&repo, "Merge", &[&branch1_commit, &branch2_commit], Some("HEAD"))?;

        let walker = GitWalker::new(Some(repo.path().to_str().unwrap()))?;
        let dag = walker.into_dag(None)?;

        assert_eq!(dag.node_count(), 4);
        assert_eq!(dag.edge_count(), 4); // base<-b1, base<-b2, b1<-merge, b2<-merge

        let stats = dag.stats();
        assert_eq!(stats.merge_commits, 1);
        assert_eq!(stats.root_commits, 1);

        Ok(())
    }
}