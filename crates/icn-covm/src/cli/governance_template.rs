//! Governance template CLI commands
//!
//! This module provides CLI commands for managing governance templates,
//! including listing, viewing, creating, editing, and applying templates.

use crate::cli::helpers::{load_identity_from_file, Output};
use crate::governance::templates::{FileBackedTemplateRegistry, Template, TemplateError};
use crate::identity::Identity;
use clap::{Args, Subcommand};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

/// CLI commands for managing governance templates
#[derive(Debug, Args)]
pub struct TemplateCommand {
    /// Subcommand for template operations
    #[command(subcommand)]
    pub command: TemplateSubcommand,
}

/// Subcommands for template management
#[derive(Debug, Subcommand)]
pub enum TemplateSubcommand {
    /// List all available templates
    List {
        /// Show detailed information for each template
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// View a specific template
    View {
        /// Template ID to view
        id: String,
        
        /// Show detailed information including all parameters
        #[arg(short, long)]
        verbose: bool,
        
        /// Show previous versions
        #[arg(short, long)]
        history: bool,
    },
    
    /// Create a new template
    Create {
        /// Template name
        name: String,
        
        /// Template file in JSON format
        #[arg(short, long)]
        file: PathBuf,
        
        /// Identity file for signing
        #[arg(short, long)]
        identity: PathBuf,
    },
    
    /// Edit an existing template
    Edit {
        /// Template ID to edit
        id: String,
        
        /// New template file in JSON format
        #[arg(short, long)]
        file: PathBuf,
        
        /// Identity file for signing
        #[arg(short, long)]
        identity: PathBuf,
    },
    
    /// Apply a template to create a new proposal
    Apply {
        /// Template ID to apply
        id: String,
        
        /// Parameters file in JSON format
        #[arg(short, long)]
        params: PathBuf,
        
        /// Identity file for signing
        #[arg(short, long)]
        identity: PathBuf,
    },
}

/// Execute template CLI commands
pub fn execute(cmd: TemplateCommand, templates_dir: PathBuf) -> Output {
    match cmd.command {
        TemplateSubcommand::List { verbose } => list_templates(templates_dir, verbose),
        TemplateSubcommand::View { id, verbose, history } => view_template(templates_dir, id, verbose, history),
        TemplateSubcommand::Create { name, file, identity } => {
            create_template(templates_dir, name, file, identity)
        }
        TemplateSubcommand::Edit { id, file, identity } => {
            edit_template(templates_dir, id, file, identity)
        }
        TemplateSubcommand::Apply { id, params, identity } => {
            apply_template(templates_dir, id, params, identity)
        }
    }
}

/// List all available templates
fn list_templates(templates_dir: PathBuf, verbose: bool) -> Output {
    let registry = match FileBackedTemplateRegistry::new(&templates_dir) {
        Ok(reg) => reg,
        Err(err) => {
            return Output::error(format!("Failed to initialize template registry: {}", err));
        }
    };
    
    let templates = match registry.list_templates() {
        Ok(templates) => templates,
        Err(err) => {
            return Output::error(format!("Failed to list templates: {}", err));
        }
    };
    
    if templates.is_empty() {
        return Output::warning("No templates found.");
    }
    
    let mut output = String::new();
    output.push_str(&format!("{} available templates:\n\n", templates.len()));
    
    for template in templates {
        output.push_str(&format!("{}: {}\n", template.id.bold(), template.name));
        output.push_str(&format!("  Version: {}\n", template.version.version));
        output.push_str(&format!("  Author: {}\n", template.version.author));
        
        if verbose {
            output.push_str(&format!("  Voting method: {:?}\n", template.voting.method));
            output.push_str(&format!("  Quorum: {}\n", template.voting.quorum));
            output.push_str(&format!("  Threshold: {}\n", template.voting.threshold));
            output.push_str(&format!("  Parameters: {}\n", template.parameters.len()));
            
            for (name, param) in &template.parameters {
                output.push_str(&format!("    {}: {:?} ({})\n", 
                    name, 
                    param.param_type,
                    if param.required { "required" } else { "optional" }
                ));
            }
        }
        
        output.push_str("\n");
    }
    
    Output::success(output)
}

/// View a specific template
fn view_template(templates_dir: PathBuf, id: String, verbose: bool, history: bool) -> Output {
    let registry = match FileBackedTemplateRegistry::new(&templates_dir) {
        Ok(reg) => reg,
        Err(err) => {
            return Output::error(format!("Failed to initialize template registry: {}", err));
        }
    };
    
    let template = match registry.get_template(&id) {
        Ok(template) => template,
        Err(err) => {
            return Output::error(format!("Failed to load template: {}", err));
        }
    };
    
    let mut output = String::new();
    output.push_str(&format!("Template: {} ({})\n\n", template.name.bold(), template.id));
    output.push_str(&format!("Version: {}\n", template.version.version));
    output.push_str(&format!("Author: {}\n", template.version.author));
    output.push_str(&format!("Created: {}\n", 
        chrono::DateTime::from_timestamp(template.version.created_at as i64, 0)
            .map(|dt| dt.to_rfc2822())
            .unwrap_or_else(|| "Unknown".to_string())
    ));
    
    output.push_str(&format!("\nVoting Configuration:\n"));
    output.push_str(&format!("  Method: {:?}\n", template.voting.method));
    output.push_str(&format!("  Quorum: {}\n", template.voting.quorum));
    output.push_str(&format!("  Threshold: {}\n", template.voting.threshold));
    output.push_str(&format!("  Deliberation: {} days\n", 
        template.voting.deliberation_period / 86400
    ));
    output.push_str(&format!("  Voting period: {} days\n", 
        template.voting.voting_period / 86400
    ));
    
    output.push_str(&format!("\nEligibility Requirements:\n"));
    if let Some(role) = &template.eligibility.required_role {
        output.push_str(&format!("  Required role: {}\n", role));
    } else {
        output.push_str("  Required role: None\n");
    }
    
    if let Some(rep) = &template.eligibility.minimum_reputation {
        output.push_str(&format!("  Minimum reputation: {}\n", rep));
    } else {
        output.push_str("  Minimum reputation: None\n");
    }
    
    output.push_str(&format!("\nParameters ({}):\n", template.parameters.len()));
    for (name, param) in &template.parameters {
        output.push_str(&format!("  {}: {:?} {}\n", 
            name, 
            param.param_type,
            if param.required { "(required)" } else { "(optional)" }
        ));
        
        if verbose {
            output.push_str(&format!("    Description: {}\n", param.description));
            if let Some(default) = &param.default_value {
                output.push_str(&format!("    Default: {}\n", default));
            }
        }
    }
    
    if verbose {
        output.push_str(&format!("\nExecution Logic:\n"));
        output.push_str("  On Approve:\n");
        for (i, op) in template.execution.on_approve.iter().enumerate() {
            output.push_str(&format!("    {}: {}\n", i + 1, op));
        }
        
        if let Some(on_reject) = &template.execution.on_reject {
            output.push_str("  On Reject:\n");
            for (i, op) in on_reject.iter().enumerate() {
                output.push_str(&format!("    {}: {}\n", i + 1, op));
            }
        }
        
        if let Some(delay) = template.execution.execution_delay {
            output.push_str(&format!("  Execution delay: {} hours\n", delay / 3600));
        }
    }
    
    if history && !template.previous_versions.is_empty() {
        output.push_str(&format!("\nVersion History:\n"));
        for (i, version) in template.previous_versions.iter().enumerate() {
            output.push_str(&format!("  Version {}:\n", version.version));
            output.push_str(&format!("    Author: {}\n", version.author));
            output.push_str(&format!("    Date: {}\n", 
                chrono::DateTime::from_timestamp(version.created_at as i64, 0)
                    .map(|dt| dt.to_rfc2822())
                    .unwrap_or_else(|| "Unknown".to_string())
            ));
            output.push_str(&format!("    Description: {}\n", version.description));
            
            if i < template.previous_versions.len() - 1 {
                output.push_str("\n");
            }
        }
    }
    
    Output::success(output)
}

/// Create a new template
fn create_template(templates_dir: PathBuf, name: String, file: PathBuf, identity_file: PathBuf) -> Output {
    // Load identity
    let identity = match load_identity_from_file(&identity_file) {
        Ok(id) => id,
        Err(err) => {
            return Output::error(format!("Failed to load identity: {}", err));
        }
    };
    
    // Read template file
    let template_json = match fs::read_to_string(&file) {
        Ok(content) => content,
        Err(err) => {
            return Output::error(format!("Failed to read template file: {}", err));
        }
    };
    
    // Parse the template
    let template: Template = match serde_json::from_str(&template_json) {
        Ok(template) => template,
        Err(err) => {
            return Output::error(format!("Failed to parse template: {}", err));
        }
    };
    
    // Create the template
    let registry = match FileBackedTemplateRegistry::new(&templates_dir) {
        Ok(reg) => reg,
        Err(err) => {
            return Output::error(format!("Failed to initialize template registry: {}", err));
        }
    };
    
    match registry.create_template(&name, &template, &identity) {
        Ok(id) => {
            Output::success(format!("Template '{}' created with ID: {}", name, id))
        }
        Err(err) => {
            Output::error(format!("Failed to create template: {}", err))
        }
    }
}

/// Edit an existing template
fn edit_template(templates_dir: PathBuf, id: String, file: PathBuf, identity_file: PathBuf) -> Output {
    // Load identity
    let identity = match load_identity_from_file(&identity_file) {
        Ok(id) => id,
        Err(err) => {
            return Output::error(format!("Failed to load identity: {}", err));
        }
    };
    
    // Read template file
    let template_json = match fs::read_to_string(&file) {
        Ok(content) => content,
        Err(err) => {
            return Output::error(format!("Failed to read template file: {}", err));
        }
    };
    
    // Parse the template
    let template: Template = match serde_json::from_str(&template_json) {
        Ok(template) => template,
        Err(err) => {
            return Output::error(format!("Failed to parse template: {}", err));
        }
    };
    
    // Update the template
    let registry = match FileBackedTemplateRegistry::new(&templates_dir) {
        Ok(reg) => reg,
        Err(err) => {
            return Output::error(format!("Failed to initialize template registry: {}", err));
        }
    };
    
    match registry.update_template(&id, &template, &identity) {
        Ok(_) => {
            Output::success(format!("Template '{}' updated successfully", id))
        }
        Err(err) => {
            Output::error(format!("Failed to update template: {}", err))
        }
    }
}

/// Apply a template to create a new proposal
fn apply_template(templates_dir: PathBuf, id: String, params_file: PathBuf, identity_file: PathBuf) -> Output {
    // Load identity
    let identity = match load_identity_from_file(&identity_file) {
        Ok(id) => id,
        Err(err) => {
            return Output::error(format!("Failed to load identity: {}", err));
        }
    };
    
    // Read parameters file
    let params_json = match fs::read_to_string(&params_file) {
        Ok(content) => content,
        Err(err) => {
            return Output::error(format!("Failed to read parameters file: {}", err));
        }
    };
    
    // Parse the parameters
    let params: serde_json::Value = match serde_json::from_str(&params_json) {
        Ok(params) => params,
        Err(err) => {
            return Output::error(format!("Failed to parse parameters: {}", err));
        }
    };
    
    // Get the template
    let registry = match FileBackedTemplateRegistry::new(&templates_dir) {
        Ok(reg) => reg,
        Err(err) => {
            return Output::error(format!("Failed to initialize template registry: {}", err));
        }
    };
    
    let template = match registry.get_template(&id) {
        Ok(template) => template,
        Err(err) => {
            return Output::error(format!("Failed to load template: {}", err));
        }
    };
    
    // TODO: Implement proposal creation from template
    // For now, we'll just return a message
    Output::success(format!(
        "Template '{}' would be applied with parameters:\n{}",
        template.name,
        serde_json::to_string_pretty(&params).unwrap_or_else(|_| params_json)
    ))
} 