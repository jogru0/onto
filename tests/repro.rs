use std::{
    fs::{remove_dir_all, File},
    io::ErrorKind,
    path::Path,
    process::Command,
};

use anyhow::Error;
use git2::{Commit, Index, ObjectType, Repository, Signature};

fn git(path: &str) -> Command {
    let mut command = Command::new("git");
    command.current_dir(path);
    command
}

fn git_init(path: &str) -> std::result::Result<(), Error> {
    let status = git(".").arg("init").arg(path).spawn()?.wait()?;
    if !(status.success()) {
        Err(Error::msg("git init failed"))
    } else {
        Ok(())
    }
}

fn git_init_2(path: &str) -> std::result::Result<Repository, git2::Error> {
    git2::Repository::init(path)
}

fn git_create_branch(path: &str, name: &str) -> std::result::Result<(), Error> {
    let status = git(path)
        .arg("switch")
        .arg("-c")
        .arg(name)
        .spawn()?
        .wait()?;
    if !(status.success()) {
        Err(Error::msg("git switch failed"))
    } else {
        Ok(())
    }
}

fn git_switch_to_branch(path: &str, name: &str) -> std::result::Result<(), Error> {
    let status = git(path).arg("switch").arg(name).spawn()?.wait()?;
    if !(status.success()) {
        Err(Error::msg("git switch failed"))
    } else {
        Ok(())
    }
}

fn git_rebase(path: &str, name: &str) -> std::result::Result<(), Error> {
    let status = git(path).arg("rebase").arg(name).spawn()?.wait()?;
    if !(status.success()) {
        Err(Error::msg("git rebase failed"))
    } else {
        Ok(())
    }
}

fn git_commit(path: &str, name: &str) -> std::result::Result<(), Error> {
    let file_name = format!("{path}/{name}");
    File::create_new(file_name).unwrap();

    let status = git(path).arg("add").arg(name).spawn()?.wait()?;
    if !(status.success()) {
        return Err(Error::msg("git add failed"));
    }

    let status = git(path)
        .arg("commit")
        .arg("-m")
        .arg(name)
        .spawn()?
        .wait()?;
    if !(status.success()) {
        Err(Error::msg("git commit failed"))
    } else {
        Ok(())
    }
}

fn find_last_commit(repo: &Repository) -> Result<Option<Commit>, git2::Error> {
    let Ok(head) = repo.head() else {
        return Ok(None);
    };

    let obj = head.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit()
        .map(Some)
        .map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

fn git_commit_2(
    repo: &mut Repository,
    name: &str,
) -> std::result::Result<git2::Oid, anyhow::Error> {
    let path = repo.workdir().ok_or(Error::msg("no worktree found"))?;
    let file_name = path.join(name);
    File::create_new(file_name)?;

    let mut index = repo.index()?;
    index.add_path(Path::new(name))?;
    let oid = index.write_tree()?;
    index.clear()?;
    index.write()?;

    let signature = repo.signature()?;
    let maybe_parent = find_last_commit(repo)?;
    let maybe_parent_ref = maybe_parent.as_ref();
    let parents = maybe_parent_ref.as_slice();
    let tree = repo.find_tree(oid)?;
    Ok(repo.commit(
        Some("HEAD"), //  point HEAD to our new commit
        &signature,   // author
        &signature,   // committer
        name,         // commit message
        &tree,        // tree
        parents,
    )?) // parents
}

pub fn remove_dir_if_found(dir: &str) -> Result<(), std::io::Error> {
    assert!(dir.starts_with("out/"));
    remove_dir_all(dir).or_else(|err| {
        if err.kind() == ErrorKind::NotFound {
            Ok(())
        } else {
            Err(err)
        }
    })
}

fn git_what(repo: &mut Repository, name: &str) -> std::result::Result<(), anyhow::Error> {
    // let path = repo.workdir().ok_or(Error::msg("no worktree found"))?;
    // let file_name = path.join(name);
    // File::create_new(file_name)?;

    repo.checkout_index(None, None).unwrap();

    let mut index = repo.index()?;
    // index.add_path(Path::new(name))?;
    // index.write()?;

    Ok(())
}

#[test]
fn repro_issue() {
    let main = "main";
    let branch_1 = "branch_1";
    let branch_2 = "branch_2";

    let path = "out/repro_issue";
    remove_dir_if_found(path).unwrap();
    let mut repo = git_init_2(path).unwrap();

    git_commit(path, "a").unwrap();
    git_commit(path, "b").unwrap();
    git_commit(path, "c").unwrap();

    git_create_branch(path, branch_1).unwrap();
    git_commit(path, "alpha").unwrap();
    git_commit(path, "beta").unwrap();

    git_create_branch(path, branch_2).unwrap();
    git_commit(path, "v").unwrap();
    git_commit(path, "w").unwrap();

    git_switch_to_branch(path, main).unwrap();
    git_what(&mut repo, "d").unwrap();
    // git_commit(path, "d").unwrap();
    // git_commit(path, "e").unwrap();

    // git_switch_to_branch(path, branch_1).unwrap();
    // git_commit(path, "gamma").unwrap();
    // git_commit(path, "delta").unwrap();

    // git_switch_to_branch(path, branch_2).unwrap();
    // git_commit(path, "x").unwrap();
    // git_commit(path, "y").unwrap();

    // git_switch_to_branch(path, branch_1).unwrap();
    // git_rebase(path, main).unwrap();
    // git_commit(path, "lambda").unwrap();
    // git_commit(path, "sigma").unwrap();
    // git_commit(path, "omega").unwrap();

    // git_switch_to_branch(path, branch_2).unwrap();
    // git_commit(path, "z").unwrap();
}

#[test]
fn repro_issue_2() {
    let main = "main";
    let branch_1 = "branch_1";
    let branch_2 = "branch_2";

    let path = "out/repro_issue_2";
    remove_dir_if_found(path).unwrap();
    let mut repo = git_init_2(path).unwrap();

    git_commit_2(&mut repo, "a").unwrap();
    git_commit_2(&mut repo, "b").unwrap();
    git_commit_2(&mut repo, "c").unwrap();

    git_create_branch(path, branch_1).unwrap();
    git_commit_2(&mut repo, "alpha").unwrap();
    git_commit_2(&mut repo, "beta").unwrap();

    git_create_branch(path, branch_2).unwrap();
    git_commit_2(&mut repo, "v").unwrap();
    git_commit_2(&mut repo, "w").unwrap();

    git_switch_to_branch(path, main).unwrap();
    git_what(&mut repo, "d").unwrap();
    // git_commit_2(&mut repo, "e").unwrap();

    // git_switch_to_branch(path, branch_1).unwrap();
    // git_commit_2(&mut repo, "gamma").unwrap();
    // git_commit_2(&mut repo, "delta").unwrap();

    // git_switch_to_branch(path, branch_2).unwrap();
    // git_commit_2(&mut repo, "x").unwrap();
    // git_commit_2(&mut repo, "y").unwrap();

    // git_switch_to_branch(path, branch_1).unwrap();
    // git_rebase(path, main).unwrap();
    // git_commit_2(&mut repo, "lambda").unwrap();
    // git_commit_2(&mut repo, "sigma").unwrap();
    // git_commit_2(&mut repo, "omega").unwrap();

    // git_switch_to_branch(path, branch_2).unwrap();
    // git_commit(path, "z").unwrap();
}
