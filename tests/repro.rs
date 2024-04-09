use std::{
    fs::{remove_dir_all, remove_file, File},
    io::ErrorKind,
    path::Path,
    process::Command,
};

use anyhow::Error;
use assert_cmd::{assert::OutputAssertExt, cargo::CommandCargoExt};
use git2::{BranchType, Commit, ObjectType, Repository};
use predicates::prelude::predicate;

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

fn git_create_branch_2(repo: &Repository, name: &str) -> std::result::Result<(), Error> {
    if let Some(commit) = find_last_commit(repo)? {
        repo.branch(name, &commit, false)?;
    }

    repo.set_head(&format!("refs/heads/{name}"))?;

    Ok(())
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

fn git_switch_to_branch_2(repo: &Repository, name: &str) -> std::result::Result<(), Error> {
    if let Ok(branch) = repo.find_branch(name, BranchType::Local) {
        let commit = branch.get().peel_to_commit()?;
        repo.checkout_tree(commit.as_object(), None)?;
    } else {
        let mut index = repo.index()?;
        let Some(repo_path) = repo.workdir() else {
            return Err(Error::msg("workdir"));
        };

        for entry in index.iter() {
            let path = repo_path.join(String::from_utf8(entry.path)?);
            remove_file(path)?;
        }

        index.remove_all(Path::new("."), None)?;
        index.write()?;
    };

    repo.set_head(&format!("refs/heads/{name}"))?;

    Ok(())
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

fn git_rebase_2(repo: &Repository, name: &str) -> std::result::Result<(), Error> {
    let branch = repo.find_branch(name, BranchType::Local)?;
    let annotated_commit = repo.reference_to_annotated_commit(branch.get())?;

    let mut rebase = repo.rebase(None, Some(&annotated_commit), None, None)?;

    while let Some(maybe_op) = rebase.next() {
        assert!(maybe_op.is_ok_and(|op| op.kind() == Some(git2::RebaseOperationType::Pick)));
        rebase.commit(None, &repo.signature()?, None)?;
    }

    rebase.finish(None)?;

    Ok(())
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

fn git_commit_list(
    repo: &Repository,
    commit_count: &mut u32,
    number_commits: u32,
) -> std::result::Result<(), anyhow::Error> {
    let target = *commit_count + number_commits;
    while *commit_count != target {
        git_commit_2(repo, &commit_count.to_string())?;
        *commit_count += 1;
    }
    Ok(())
}

fn git_commit_2<'a>(
    repo: &'a Repository,
    name: &str,
) -> std::result::Result<Commit<'a>, anyhow::Error> {
    let path = repo.workdir().ok_or(Error::msg("no worktree found"))?;
    let file_name = path.join(name);
    File::create_new(file_name)?;

    let mut index = repo.index()?;
    index.add_path(Path::new(name))?;
    let oid = index.write_tree()?;
    index.write()?;

    let signature = repo.signature()?;
    let maybe_parent = find_last_commit(repo)?;
    let maybe_parent_ref = maybe_parent.as_ref();
    let parents = maybe_parent_ref.as_slice();
    let tree = repo.find_tree(oid)?;
    let res = repo.commit(
        Some("HEAD"), //  point HEAD to our new commit
        &signature,   // author
        &signature,   // committer
        name,         // commit message
        &tree,        // tree
        // parents
        parents,
    )?;

    Ok(repo.find_commit(res)?)
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

#[test]
fn repro_issue_2() {
    let main = "main";
    let branch_1 = "branch_1";
    let branch_2 = "branch_2";

    let path = "out/repro_issue_2";
    remove_dir_if_found(path).unwrap();
    let repo = git_init_2(path).unwrap();

    git_commit_2(&repo, "a").unwrap();
    git_commit_2(&repo, "b").unwrap();
    git_commit_2(&repo, "c").unwrap();

    git_create_branch_2(&repo, branch_1).unwrap();
    git_commit_2(&repo, "alpha").unwrap();
    git_commit_2(&repo, "beta").unwrap();

    git_create_branch_2(&repo, branch_2).unwrap();
    git_commit_2(&repo, "v").unwrap();
    git_commit_2(&repo, "w").unwrap();

    git_switch_to_branch_2(&repo, main).unwrap();
    git_commit_2(&repo, "d").unwrap();
    git_commit_2(&repo, "e").unwrap();

    git_switch_to_branch_2(&repo, branch_1).unwrap();
    git_commit_2(&repo, "gamma").unwrap();
    git_commit_2(&repo, "delta").unwrap();

    git_switch_to_branch_2(&repo, branch_2).unwrap();
    git_commit_2(&repo, "x").unwrap();
    git_commit_2(&repo, "y").unwrap();

    git_switch_to_branch_2(&repo, branch_1).unwrap();
    git_rebase_2(&repo, main).unwrap();
    git_commit_2(&repo, "lambda").unwrap();
    git_commit_2(&repo, "sigma").unwrap();
    git_commit_2(&repo, "omega").unwrap();

    git_switch_to_branch_2(&repo, branch_2).unwrap();
    git_commit_2(&repo, "z").unwrap();
}

#[test]
fn repro_issue_3() {
    let name = "repro_issue_3";

    for n_main_commits_pre in 0..4 {
        for n_branch_1_commits_pre in 0..4 {
            for n_branch_2_commits in 0..4 {
                for n_main_commits_post in 0..4 {
                    for n_branch_1_commits_post_before_rebase in 0..4 {
                        for n_branch_1_commits_post_after_rebase in 0..4 {
                            repro_issue_dynamic(
                                name.into(),
                                n_main_commits_pre,
                                n_branch_1_commits_pre,
                                n_branch_2_commits,
                                n_main_commits_post,
                                n_branch_1_commits_post_before_rebase,
                                n_branch_1_commits_post_after_rebase,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn repro_issue_dynamic(
    name: String,
    n_main_commits_pre: u32,
    n_branch_1_commits_pre: u32,
    n_branch_2_commits: u32,
    n_main_commits_post: u32,
    n_branch_1_commits_post_before_rebase: u32,
    n_branch_1_commits_post_after_rebase: u32,
) {
    let we_rebase = n_main_commits_pre != 0;

    let main = "main";
    let branch_1 = "branch_1";
    let branch_2 = "branch_2";

    let path = format!("out/{name}");
    remove_dir_if_found(&path).unwrap();
    let repo = git_init_2(&path).unwrap();

    let mut commit_count = 0;

    git_commit_list(&repo, &mut commit_count, n_main_commits_pre).unwrap();

    git_create_branch_2(&repo, branch_1).unwrap();
    git_commit_list(&repo, &mut commit_count, n_branch_1_commits_pre).unwrap();
    let maybe_expected = find_last_commit(&repo).unwrap();

    git_create_branch_2(&repo, branch_2).unwrap();
    git_commit_list(&repo, &mut commit_count, n_branch_2_commits).unwrap();

    git_switch_to_branch_2(&repo, main).unwrap();
    git_commit_list(&repo, &mut commit_count, n_main_commits_post).unwrap();

    git_switch_to_branch_2(&repo, branch_1).unwrap();
    git_commit_list(
        &repo,
        &mut commit_count,
        n_branch_1_commits_post_before_rebase,
    )
    .unwrap();

    if we_rebase {
        git_rebase_2(&repo, main).unwrap();
    }
    git_commit_list(
        &repo,
        &mut commit_count,
        n_branch_1_commits_post_after_rebase,
    )
    .unwrap();
    let expected = maybe_expected.or_else(|| find_last_commit(&repo).unwrap());

    git_switch_to_branch_2(&repo, branch_2).unwrap();

    let mut bin = Command::cargo_bin("onto").unwrap();
    bin.current_dir(&path).arg(branch_1);

    let assert = bin
        .assert()
        .append_context("name", name)
        .append_context("n_main_commits_pre", n_main_commits_pre)
        .append_context("n_branch_1_commits_pre", n_branch_1_commits_pre)
        .append_context("n_branch_2_commits", n_branch_2_commits)
        .append_context("n_main_commits_post", n_main_commits_post)
        .append_context(
            "n_branch_1_commits_post_before_rebase",
            n_branch_1_commits_post_before_rebase,
        )
        .append_context(
            "n_branch_1_commits_post_after_rebase",
            n_branch_1_commits_post_after_rebase,
        );

    let branch_2_is_unborn = n_branch_2_commits + n_branch_1_commits_pre + n_main_commits_pre == 0;
    // n_main_commits_post doesn't matter, as n_main_commits_pre == 0 already implies that we never rebased.
    let branch_1_does_not_exist = n_main_commits_pre
        + n_branch_1_commits_pre
        + n_branch_1_commits_post_before_rebase
        + n_branch_1_commits_post_after_rebase
        == 0;

    assert!(!(branch_1_does_not_exist && we_rebase));

    if branch_2_is_unborn {
        assert
            .stderr(predicate::eq("unborn\n"))
            .stdout(predicate::eq(""))
            .code(255)
            .failure();
    } else if branch_1_does_not_exist {
        assert
            .stderr(predicate::eq("branch does not exist\n"))
            .stdout(predicate::eq(""))
            .code(255)
            .failure();
    } else {
        let expected_oid = expected.unwrap().as_object().id().to_string();
        let expected_out = format!("{expected_oid}\n",);
        assert
            .stdout(predicate::eq(expected_out))
            .stderr(predicate::eq(""))
            .code(0)
            .success();

        let mut bin = Command::new("git");
        bin.current_dir(&path)
            .arg("rebase")
            .arg("--onto")
            .arg(branch_1)
            .arg(expected_oid)
            .assert()
            .stdout(predicate::function(|stdout: &str| {
                stdout.is_empty() || stdout == "Current branch branch_2 is up to date.\n"
            }))
            .stderr(predicate::function(|stderr: &str| {
                !(stderr.contains("skip") || stderr.contains("drop"))
            }))
            .code(0)
            .success();

        let mut expected_commits = n_main_commits_pre
            + n_branch_1_commits_post_after_rebase
            + n_branch_1_commits_post_before_rebase
            + n_branch_1_commits_pre
            + n_branch_2_commits;

        if we_rebase {
            expected_commits += n_main_commits_post;
        }

        let mut bin = Command::new("git");
        bin.current_dir(path)
            .arg("rev-list")
            .arg("--count")
            .arg("HEAD")
            .assert()
            .stdout(format!("{expected_commits}\n"))
            .stderr("")
            .code(0)
            .success();
    }
}
