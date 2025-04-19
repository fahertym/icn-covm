//! Governance Template Registry
//!
//! This module implements a registry for governance templates that can be used
//! to create proposals with standardized parameters and execution logic.
//!
//! Templates provide consistent governance patterns that can be reused across
//! multiple proposals, ensuring procedural fairness and transparency.

use crate::storage::traits::Storage;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::auth::AuthContext;
use crate::identity::Identity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::marker::{Send, Sync};
use std::path::PathBuf;
use std::fs;
use std::io;

/// Governance template version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVersion {
    /// Version number
    pub version: String,
    
    /// Creator of this version
    pub author: String,
    
    /// Timestamp when created
    pub created_at: u64,
    
    /// Description of this version
    pub description: String,
}

/// Governance template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Unique identifier for the template
    pub id: String,
    
    /// Human-readable name of the template
    pub name: String,
    
    /// Current version information
    pub version: TemplateVersion,
    
    /// Previous versions of this template (if any)
    pub previous_versions: Vec<TemplateVersion>,
    
    /// Template parameters definition
    pub parameters: HashMap<String, ParameterDefinition>,
    
    /// Voting configuration
    pub voting: VotingConfig,
    
    /// Eligibility requirements
    pub eligibility: EligibilityConfig,
    
    /// Execution logic as a series of VM operations
    pub execution: ExecutionConfig,
}

/// Definition of a parameter that can be provided when creating a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    /// Parameter name
    pub name: String,
    
    /// Parameter description
    pub description: String,
    
    /// Parameter type
    pub param_type: ParameterType,
    
    /// Whether this parameter is required
    pub required: bool,
    
    /// Default value if not provided
    pub default_value: Option<String>,
}

/// Parameter types for template parameters
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParameterType {
    /// String parameter
    String,
    
    /// Numeric parameter
    Number,
    
    /// Boolean parameter
    Boolean,
    
    /// Identity parameter
    Identity,
    
    /// Resource parameter
    Resource,
}

/// Voting configuration for a governance template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingConfig {
    /// Quorum threshold (minimum participation)
    pub quorum: f64,
    
    /// Approval threshold (minimum votes in favor)
    pub threshold: f64,
    
    /// Voting method
    pub method: VotingMethod,
    
    /// Deliberation period in seconds
    pub deliberation_period: u64,
    
    /// Voting period in seconds
    pub voting_period: u64,
}

/// Methods for vote counting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VotingMethod {
    /// One member, one vote
    SimpleMajority,
    
    /// Weighted by reputation
    ReputationWeighted,
    
    /// Ranked choice voting
    RankedChoice,
}

/// Configuration for who can participate in voting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityConfig {
    /// Required role to vote
    pub required_role: Option<String>,
    
    /// Minimum reputation to vote
    pub minimum_reputation: Option<f64>,
    
    /// Custom eligibility logic as VM operations
    pub custom_logic: Option<Vec<String>>,
}

/// Configuration for proposal execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// VM operations to execute when approved
    pub on_approve: Vec<String>,
    
    /// VM operations to execute when rejected
    pub on_reject: Option<Vec<String>>,
    
    /// Delay after approval before execution
    pub execution_delay: Option<u64>,
}

/// Errors that can occur in template operations
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    /// Template not found
    #[error("Template not found: {id}")]
    TemplateNotFound { id: String },
    
    /// Invalid template format
    #[error("Invalid template format: {details}")]
    InvalidFormat { details: String },
    
    /// Permission denied
    #[error("Permission denied: {details}")]
    PermissionDenied { details: String },
    
    /// Storage error
    #[error("Storage error: {details}")]
    StorageError { details: String },
    
    /// I/O error
    #[error("I/O error: {details}")]
    IoError { details: String },
}

impl From<StorageError> for TemplateError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::ResourceNotFound { key, .. } => {
                TemplateError::TemplateNotFound { id: key }
            }
            StorageError::PermissionDenied { action, .. } => {
                TemplateError::PermissionDenied { details: action }
            }
            _ => TemplateError::StorageError {
                details: error.to_string(),
            },
        }
    }
}

impl From<io::Error> for TemplateError {
    fn from(error: io::Error) -> Self {
        TemplateError::IoError {
            details: error.to_string(),
        }
    }
}

/// Result type for template operations
pub type TemplateResult<T> = Result<T, TemplateError>;

/// Registry for governance templates
pub struct TemplateRegistry<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Storage backend
    storage: S,
    
    /// Template storage path for file-backed storage
    templates_path: Option<PathBuf>,
}

impl<S> TemplateRegistry<S>
where
    S: Storage + Send + Sync + Clone + Debug + 'static,
{
    /// Create a new template registry with the given storage backend
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            templates_path: None,
        }
    }
    
    /// Set the file path for template storage
    pub fn with_templates_path(mut self, path: PathBuf) -> Self {
        self.templates_path = Some(path);
        self
    }
    
    /// Ensure the templates directory exists
    fn ensure_templates_dir(&self) -> TemplateResult<()> {
        if let Some(path) = &self.templates_path {
            if !path.exists() {
                fs::create_dir_all(path)?;
            }
        }
        Ok(())
    }
    
    /// Create a new template
    pub fn create_template(
        &mut self,
        name: &str,
        definition: &Template,
        author: &Identity,
        auth_context: Option<&AuthContext>,
    ) -> TemplateResult<String> {
        // Generate a unique ID
        let id = format!("template:{}", uuid::Uuid::new_v4());
        
        // Store in storage backend
        let key = format!("templates:{}", id);
        let value = serde_json::to_string(definition)
            .map_err(|e| TemplateError::InvalidFormat { details: e.to_string() })?;
        
        self.storage.store_string(&key, &value, auth_context, "governance")
            .map_err(TemplateError::from)?;
        
        // If file storage is enabled, also store there
        if let Some(path) = &self.templates_path {
            self.ensure_templates_dir()?;
            let file_path = path.join(format!("{}.json", id));
            fs::write(file_path, value)?;
        }
        
        Ok(id)
    }
    
    /// Get a template by ID
    pub fn get_template(
        &self,
        id: &str,
        auth_context: Option<&AuthContext>,
    ) -> TemplateResult<Template> {
        // Try to get from storage backend
        let key = format!("templates:{}", id);
        let value = self.storage.load_string(&key, auth_context, "governance")
            .map_err(TemplateError::from)?;
        
        // Deserialize the template
        serde_json::from_str(&value)
            .map_err(|e| TemplateError::InvalidFormat { details: e.to_string() })
    }
    
    /// List all templates
    pub fn list_templates(
        &self,
        auth_context: Option<&AuthContext>,
    ) -> TemplateResult<Vec<Template>> {
        // Get all keys matching the template pattern
        let prefix = "templates:";
        let keys = self.storage.keys_with_prefix(prefix, auth_context, "governance")
            .map_err(TemplateError::from)?;
        
        // Load each template
        let mut templates = Vec::new();
        for key in keys {
            let value = self.storage.load_string(&key, auth_context, "governance")
                .map_err(TemplateError::from)?;
            
            let template = serde_json::from_str(&value)
                .map_err(|e| TemplateError::InvalidFormat { details: e.to_string() })?;
            
            templates.push(template);
        }
        
        Ok(templates)
    }
    
    /// Update an existing template
    pub fn update_template(
        &mut self,
        id: &str,
        updated_definition: &Template,
        author: &Identity,
        auth_context: Option<&AuthContext>,
    ) -> TemplateResult<()> {
        // Get the existing template
        let mut template = self.get_template(id, auth_context)?;
        
        // Store the current version in previous versions
        template.previous_versions.push(template.version.clone());
        
        // Update with new definition
        let key = format!("templates:{}", id);
        let value = serde_json::to_string(updated_definition)
            .map_err(|e| TemplateError::InvalidFormat { details: e.to_string() })?;
        
        self.storage.store_string(&key, &value, auth_context, "governance")
            .map_err(TemplateError::from)?;
        
        // If file storage is enabled, also update there
        if let Some(path) = &self.templates_path {
            self.ensure_templates_dir()?;
            let file_path = path.join(format!("{}.json", id));
            fs::write(file_path, value)?;
        }
        
        Ok(())
    }
    
    /// Delete a template
    pub fn delete_template(
        &mut self,
        id: &str,
        auth_context: Option<&AuthContext>,
    ) -> TemplateResult<()> {
        // Delete from storage backend
        let key = format!("templates:{}", id);
        self.storage.delete(&key, auth_context, "governance")
            .map_err(TemplateError::from)?;
        
        // If file storage is enabled, also delete there
        if let Some(path) = &self.templates_path {
            let file_path = path.join(format!("{}.json", id));
            if file_path.exists() {
                fs::remove_file(file_path)?;
            }
        }
        
        Ok(())
    }
}

// Public exports
pub use self::registry::FileBackedTemplateRegistry;

// Sub-modules
mod registry; 