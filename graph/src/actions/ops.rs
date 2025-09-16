use anyhow::Result;

pub enum GraphAction {
    Checkout(String),
    CherryPick(String),
    Revert(String),
}

pub struct ActionExecutor {
    repo_path: String,
}

impl ActionExecutor {
    pub fn execute(&self, action: GraphAction) -> Result<()> {
        match action {
            GraphAction::Checkout(commit) => {
                println!("Would checkout: {}", commit);
                // git2 checkout implementation
            }
            GraphAction::CherryPick(commit) => {
                println!("Would cherry-pick: {}", commit);
            }
            GraphAction::Revert(commit) => {
                println!("Would revert: {}", commit);
            }
        }
        Ok(())
    }

    pub fn can_execute(&self) -> Result<bool> {
        // Check working directory is clean
        Ok(true)
    }
}
