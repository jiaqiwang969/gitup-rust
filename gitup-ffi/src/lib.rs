use gitup_core::{Repository, CommitInfo, BranchInfo};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Opaque handle for Repository
pub struct GitUpRepository {
    inner: Repository,
}

/// C-compatible commit info
#[repr(C)]
pub struct CCommitInfo {
    pub id: *mut c_char,
    pub message: *mut c_char,
    pub author: *mut c_char,
    pub email: *mut c_char,
    pub timestamp: i64,
}

/// C-compatible branch info
#[repr(C)]
pub struct CBranchInfo {
    pub name: *mut c_char,
    pub commit_id: *mut c_char,
    pub is_head: bool,
    pub is_remote: bool,
}

/// Open a repository at the given path
#[no_mangle]
pub extern "C" fn gitup_repository_open(path: *const c_char) -> *mut GitUpRepository {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    match Repository::open(path) {
        Ok(repo) => Box::into_raw(Box::new(GitUpRepository { inner: repo })),
        Err(_) => ptr::null_mut(),
    }
}

/// Initialize a new repository
#[no_mangle]
pub extern "C" fn gitup_repository_init(path: *const c_char) -> *mut GitUpRepository {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    match Repository::init(path) {
        Ok(repo) => Box::into_raw(Box::new(GitUpRepository { inner: repo })),
        Err(_) => ptr::null_mut(),
    }
}

/// Free a repository handle
#[no_mangle]
pub extern "C" fn gitup_repository_free(repo: *mut GitUpRepository) {
    if !repo.is_null() {
        unsafe {
            let _ = Box::from_raw(repo);
        }
    }
}

/// Check if repository is clean
#[no_mangle]
pub extern "C" fn gitup_repository_is_clean(repo: *mut GitUpRepository) -> bool {
    if repo.is_null() {
        return false;
    }

    let repo = unsafe { &(*repo).inner };
    repo.is_clean().unwrap_or(false)
}

/// Get recent commits
#[no_mangle]
pub extern "C" fn gitup_repository_get_commits(
    repo: *mut GitUpRepository,
    count: usize,
    out_commits: *mut *mut CCommitInfo,
    out_count: *mut usize,
) -> bool {
    if repo.is_null() || out_commits.is_null() || out_count.is_null() {
        return false;
    }

    let repo = unsafe { &(*repo).inner };

    match repo.get_commits(count) {
        Ok(commits) => {
            let c_commits: Vec<CCommitInfo> = commits
                .into_iter()
                .map(|commit| CCommitInfo {
                    id: CString::new(commit.id).unwrap().into_raw(),
                    message: CString::new(commit.message).unwrap().into_raw(),
                    author: CString::new(commit.author).unwrap().into_raw(),
                    email: CString::new(commit.email).unwrap().into_raw(),
                    timestamp: commit.timestamp,
                })
                .collect();

            unsafe {
                *out_count = c_commits.len();
                *out_commits = Box::into_raw(c_commits.into_boxed_slice()) as *mut CCommitInfo;
            }
            true
        }
        Err(_) => false,
    }
}

/// Free commit info array
#[no_mangle]
pub extern "C" fn gitup_commits_free(commits: *mut CCommitInfo, count: usize) {
    if commits.is_null() {
        return;
    }

    unsafe {
        for i in 0..count {
            let commit = &(*commits.add(i));
            if !commit.id.is_null() {
                let _ = CString::from_raw(commit.id);
            }
            if !commit.message.is_null() {
                let _ = CString::from_raw(commit.message);
            }
            if !commit.author.is_null() {
                let _ = CString::from_raw(commit.author);
            }
            if !commit.email.is_null() {
                let _ = CString::from_raw(commit.email);
            }
        }
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(commits, count));
    }
}

/// List all branches
#[no_mangle]
pub extern "C" fn gitup_repository_list_branches(
    repo: *mut GitUpRepository,
    out_branches: *mut *mut CBranchInfo,
    out_count: *mut usize,
) -> bool {
    if repo.is_null() || out_branches.is_null() || out_count.is_null() {
        return false;
    }

    let repo = unsafe { &(*repo).inner };

    match repo.list_branches() {
        Ok(branches) => {
            let c_branches: Vec<CBranchInfo> = branches
                .into_iter()
                .map(|branch| CBranchInfo {
                    name: CString::new(branch.name).unwrap().into_raw(),
                    commit_id: CString::new(branch.commit_id).unwrap().into_raw(),
                    is_head: branch.is_head,
                    is_remote: branch.is_remote,
                })
                .collect();

            unsafe {
                *out_count = c_branches.len();
                *out_branches = Box::into_raw(c_branches.into_boxed_slice()) as *mut CBranchInfo;
            }
            true
        }
        Err(_) => false,
    }
}

/// Free branch info array
#[no_mangle]
pub extern "C" fn gitup_branches_free(branches: *mut CBranchInfo, count: usize) {
    if branches.is_null() {
        return;
    }

    unsafe {
        for i in 0..count {
            let branch = &(*branches.add(i));
            if !branch.name.is_null() {
                let _ = CString::from_raw(branch.name);
            }
            if !branch.commit_id.is_null() {
                let _ = CString::from_raw(branch.commit_id);
            }
        }
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(branches, count));
    }
}
