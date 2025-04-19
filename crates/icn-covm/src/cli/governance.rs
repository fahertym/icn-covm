//! Governance CLI commands
//!
//! This module provides CLI commands for working with the governance system,
//! including proposal management, voting, and template handling.

use crate::cli::helpers::Output;
use crate::cli::governance_template::{TemplateCommand, execute as execute_template};
use clap::{Args, Subcommand};
use std::path::PathBuf;

/// CLI commands for governance operations
#[derive(Debug, Args)]
pub struct GovernanceCommand {
    /// Subcommand for governance operations
    #[command(subcommand)]
    pub command: GovernanceSubcommand,
}

/// Subcommands for governance operations
#[derive(Debug, Subcommand)]
pub enum GovernanceSubcommand {
    /// Governance proposal commands
    Proposal {
        /// Proposal subcommand
        #[command(subcommand)]
        command: ProposalSubcommand,
    },
    
    /// Governance template commands
    Template(TemplateCommand),
    
    /// Vote on a proposal
    Vote {
        /// Proposal ID
        id: String,
        
        /// Vote value (approve, reject, abstain)
        vote: String,
        
        /// Identity file for signing
        #[arg(short, long)]
        identity: PathBuf,
        
        /// Optional comment on vote
        #[arg(short, long)]
        comment: Option<String>,
    },
}

/// Subcommands for proposal operations
#[derive(Debug, Subcommand)]
pub enum ProposalSubcommand {
    /// Create a new proposal
    Create {
        /// Title of the proposal
        title: String,
        
        /// Description of the proposal
        description: String,
        
        /// Program file (JSON or DSL)
        #[arg(short, long)]
        program: PathBuf,
        
        /// Template ID to use (optional)
        #[arg(short, long)]
        template: Option<String>,
        
        /// Identity file for signing
        #[arg(short, long)]
        identity: PathBuf,
    },
    
    /// List all proposals
    List {
        /// Filter by status (active, pending, approved, rejected, all)
        #[arg(short, long, default_value = "active")]
        status: String,
        
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// View a proposal
    View {
        /// Proposal ID
        id: String,
        
        /// Show detailed information including votes
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Execute an approved proposal
    Execute {
        /// Proposal ID
        id: String,
        
        /// Identity file for signing
        #[arg(short, long)]
        identity: PathBuf,
    },
}

/// Execute governance CLI commands
pub fn execute(cmd: GovernanceCommand, data_dir: PathBuf) -> Output {
    match cmd.command {
        GovernanceSubcommand::Proposal { command } => {
            execute_proposal(command, data_dir)
        },
        GovernanceSubcommand::Template(template_cmd) => {
            // Templates are stored in a subdirectory of the data directory
            let templates_dir = data_dir.join("templates");
            execute_template(template_cmd, templates_dir)
        },
        GovernanceSubcommand::Vote { id, vote, identity, comment } => {
            execute_vote(id, vote, identity, comment, data_dir)
        },
    }
}

/// Execute proposal subcommands
fn execute_proposal(cmd: ProposalSubcommand, data_dir: PathBuf) -> Output {
    match cmd {
        ProposalSubcommand::Create { title, description, program, template, identity } => {
            // TODO: Implement proposal creation
            Output::info("Proposal creation not yet implemented")
        },
        ProposalSubcommand::List { status, verbose } => {
            // TODO: Implement proposal listing
            Output::info("Proposal listing not yet implemented")
        },
        ProposalSubcommand::View { id, verbose } => {
            // TODO: Implement proposal viewing
            Output::info(format!("Proposal viewing not yet implemented for ID: {}", id))
        },
        ProposalSubcommand::Execute { id, identity } => {
            // TODO: Implement proposal execution
            Output::info(format!("Proposal execution not yet implemented for ID: {}", id))
        },
    }
}

/// Execute vote command
fn execute_vote(id: String, vote: String, identity: PathBuf, comment: Option<String>, data_dir: PathBuf) -> Output {
    // TODO: Implement voting
    Output::info(format!("Voting not yet implemented for proposal ID: {}", id))
} 