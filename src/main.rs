use std::env;

use anyhow::{Error, Ok};
use git2::{Repository, Sort};

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();

    let repo = Repository::open(".")?;

    let onto = repo.find_branch(&args[1], git2::BranchType::Local)?;
    let onto = onto.get().peel(git2::ObjectType::Any)?.id();

    let branch = repo.head()?.peel(git2::ObjectType::Any)?.id();

    let merge_base = repo.merge_base(onto, branch)?;
    let merge_base_2 = repo.merge_base(branch, merge_base)?;

    dbg!(merge_base);
    dbg!(merge_base_2);

    let mut revwalk = repo.revwalk()?;

    revwalk.push(branch)?;
    revwalk.hide(onto)?;

    revwalk.set_sorting(Sort::REVERSE)?;

    let mut revwalk_onto = repo.revwalk()?;

    revwalk_onto.push(onto)?;
    revwalk_onto.hide(branch)?;

    revwalk_onto.set_sorting(Sort::REVERSE)?;

    let mut fork_point = merge_base;

    let oid = revwalk.next().unwrap().unwrap();
    let commit = repo.find_commit(oid).unwrap();

    for rev_onto in revwalk_onto.by_ref() {
        let oid_onto = rev_onto?;

        let commit_onto = repo.find_commit(oid_onto)?;

        if commit.author() == commit_onto.author() && commit.message() == commit_onto.message() {
            dbg!(oid);
            dbg!(oid_onto);
            break;
        }
    }

    while let (Some(rev), Some(rev_onto)) = (revwalk.next(), revwalk_onto.next()) {
        let oid = rev?;
        let oid_onto = rev_onto?;

        let commit = repo.find_commit(oid)?;
        let commit_onto = repo.find_commit(oid_onto)?;

        if commit.author() != commit_onto.author() || commit.message() != commit_onto.message() {
            dbg!(oid);
            dbg!(oid_onto);
            break;
        }

        fork_point = oid;
    }

    dbg!(fork_point);

    Ok(())
    // println!("Hello, world!");
    // for arg in args {
    //     println!("Argument: '{arg}'");
    // }
}
