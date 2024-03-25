use std::{
    fs::{remove_dir_all, File},
    io::ErrorKind,
    process::Command,
};

use anyhow::Error;

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
        .arg(format!("\"{name}\""))
        .spawn()?
        .wait()?;
    if !(status.success()) {
        Err(Error::msg("git commit failed"))
    } else {
        Ok(())
    }
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

#[test]
fn repro_issue() {
    let main = "main";
    let branch_1 = "branch_1";
    let branch_2 = "branch_2";

    let path = "out/repro_issue";
    remove_dir_if_found(path).unwrap();
    git_init(path).unwrap();

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
    git_commit(path, "d").unwrap();
    git_commit(path, "e").unwrap();

    git_switch_to_branch(path, branch_1).unwrap();
    git_commit(path, "gamma").unwrap();
    git_commit(path, "delta").unwrap();

    git_switch_to_branch(path, branch_2).unwrap();
    git_commit(path, "x").unwrap();
    git_commit(path, "y").unwrap();

    git_switch_to_branch(path, branch_1).unwrap();
    git_rebase(path, main).unwrap();
    git_commit(path, "lambda").unwrap();
    git_commit(path, "sigma").unwrap();
    git_commit(path, "omega").unwrap();

    git_switch_to_branch(path, branch_2).unwrap();
    git_commit(path, "z").unwrap();
}
