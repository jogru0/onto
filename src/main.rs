use std::process::exit;

use anyhow::Error;
use clap::Parser;
use git2::{ErrorClass, ErrorCode, Oid, Repository, Sort};
use log::{debug, info};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    base_branch: String,
}

fn find_old_base(repo: &Repository, branch: Oid, onto: Oid) -> Result<Oid, Error> {
    let maybe_merge_base = match repo.merge_base(onto, branch) {
        Ok(merge_base) => Some(merge_base),
        Err(err) => {
            assert_eq!(err.class(), ErrorClass::Merge);
            assert_eq!(err.code(), ErrorCode::NotFound);
            None
        }
    };

    let mut revwalk = repo.revwalk()?;

    revwalk.push(branch)?;
    revwalk.hide(onto)?;

    revwalk.set_sorting(Sort::REVERSE)?;

    let mut revwalk_onto = repo.revwalk()?;

    revwalk_onto.push(onto)?;
    revwalk_onto.hide(branch)?;

    revwalk_onto.set_sorting(Sort::REVERSE)?;

    let mut result = maybe_merge_base.unwrap_or(onto);

    debug!("fallback result is {result:?}");

    let oid = match revwalk.next() {
        Some(oid) => oid.unwrap(),
        None => {
            // Nothing was added since the merge_base, so we already found the most recent shared commit.
            // (If no merge base exists, that would imply that branch is unborn, contradicting that it is given via an Oid.)
            assert!(maybe_merge_base.is_some());
            return Ok(result);
        }
    };

    debug!("searching for a match for {oid}");

    let commit = repo.find_commit(oid).unwrap();

    for rev_onto in revwalk_onto.by_ref() {
        let oid_onto = rev_onto?;

        let commit_onto = repo.find_commit(oid_onto)?;

        if commit.author() == commit_onto.author() && commit.message() == commit_onto.message() {
            debug!("match: {oid_onto}");
            debug!("result updated to {oid}");
            result = oid;
            break;
        }

        debug!("not a match: {oid_onto}");
    }

    debug!("walking parallel as long as matching");

    while let (Some(rev), Some(rev_onto)) = (revwalk.next(), revwalk_onto.next()) {
        let oid = rev?;
        let oid_onto = rev_onto?;

        let commit = repo.find_commit(oid)?;
        let commit_onto = repo.find_commit(oid_onto)?;

        if commit.author() != commit_onto.author() || commit.message() != commit_onto.message() {
            debug!("not a match:\n\t{oid}\n\t{oid_onto}");
            break;
        }

        debug!("match:\n\t{oid}\n\t{oid_onto}");
        debug!("result updated to {oid}");
        result = oid;
    }

    Ok(result)
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    env_logger::init();

    let repo = Repository::open(".")?;

    let head = match repo.head() {
        Ok(head) => head,
        Err(err) => {
            if err.class() == ErrorClass::Reference && err.code() == ErrorCode::UnbornBranch {
                eprintln!("HEAD is an unborn branch");
                exit(-1);
            }
            return Err(err.into());
        }
    };

    if !head.is_branch() {
        eprintln!("HEAD is not a branch");
        exit(-1);
    }

    let branch = match head.peel(git2::ObjectType::Any) {
        Ok(obj) => obj.id(),
        Err(err) => {
            return Err(err.into());
        }
    };

    let onto = match repo.find_branch(&args.base_branch, git2::BranchType::Local) {
        Ok(onto) => onto,
        Err(err) => {
            if err.class() == ErrorClass::Reference && err.code() == ErrorCode::NotFound {
                eprintln!("branch '{}' does not exist", args.base_branch);
                exit(128);
            }
            return Err(err.into());
        }
    };
    let onto = onto.get().peel(git2::ObjectType::Any)?.id();

    info!("branch at {branch}");
    info!("onto at {onto}");

    println!("{}", find_old_base(&repo, branch, onto)?);
    Ok(())
}
