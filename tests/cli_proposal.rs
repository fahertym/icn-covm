use assert_cmd::Command;
use predicates::prelude::*;
use std::error::Error;
use tempfile::TempDir;

#[test]
fn test_create_proposal() -> Result<(), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_str().unwrap();

    let mut cmd = Command::cargo_bin("icn-covm")?;
    cmd.arg("proposal")
        .arg("create")
        .arg("prop-001")
        .arg("--title")
        .arg("Test Proposal")
        .arg("--author")
        .arg("test_user")
        .arg("--quorum")
        .arg("0.75")
        .arg("--threshold")
        .arg("0.6")
        .arg("--storage-backend")
        .arg("memory")
        .arg("--storage-path")
        .arg(storage_path);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Proposal created"));

    Ok(())
}

#[test]
fn test_attach_document() -> Result<(), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_str().unwrap();

    // First create a proposal
    let mut cmd = Command::cargo_bin("icn-covm")?;
    cmd.arg("proposal")
        .arg("create")
        .arg("prop-002")
        .arg("--storage-backend")
        .arg("memory")
        .arg("--storage-path")
        .arg(storage_path)
        .assert()
        .success();

    // Then attach a document
    let mut cmd = Command::cargo_bin("icn-covm")?;
    cmd.arg("proposal")
        .arg("attach")
        .arg("prop-002")
        .arg("summary")
        .arg("This is a test proposal summary")
        .arg("--storage-backend")
        .arg("memory")
        .arg("--storage-path")
        .arg(storage_path);

    cmd.assert().success();

    Ok(())
}

#[test]
fn test_vote_on_proposal() -> Result<(), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_str().unwrap();

    // First create a proposal
    let mut cmd = Command::cargo_bin("icn-covm")?;
    cmd.arg("proposal")
        .arg("create")
        .arg("prop-003")
        .arg("--storage-backend")
        .arg("memory")
        .arg("--storage-path")
        .arg(storage_path)
        .assert()
        .success();

    // Then vote on it
    let mut cmd = Command::cargo_bin("icn-covm")?;
    cmd.arg("proposal")
        .arg("vote")
        .arg("prop-003")
        .arg("--ranked")
        .arg("3")
        .arg("1")
        .arg("2")
        .arg("--storage-backend")
        .arg("memory")
        .arg("--storage-path")
        .arg(storage_path);

    cmd.assert().success();

    Ok(())
}

#[test]
fn test_invalid_proposal_id() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("icn-covm")?;
    cmd.arg("proposal")
        .arg("vote")
        .arg("nonexistent-prop")
        .arg("--ranked")
        .arg("1");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not found"));

    Ok(())
}

#[test]
fn test_invalid_vote_ranks() -> Result<(), Box<dyn Error>> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().to_str().unwrap();

    // First create a proposal
    let mut cmd = Command::cargo_bin("icn-covm")?;
    cmd.arg("proposal")
        .arg("create")
        .arg("prop-004")
        .arg("--storage-backend")
        .arg("memory")
        .arg("--storage-path")
        .arg(storage_path)
        .assert()
        .success();

    // Try to vote with invalid ranks
    let mut cmd = Command::cargo_bin("icn-covm")?;
    cmd.arg("proposal")
        .arg("vote")
        .arg("prop-004")
        .arg("--ranked")
        .arg("0") // Invalid rank (should start from 1)
        .arg("--storage-backend")
        .arg("memory")
        .arg("--storage-path")
        .arg(storage_path);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid rank"));

    Ok(())
} 