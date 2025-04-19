//! File-backed template registry implementation
//!
//! This module provides a template registry implementation that stores templates
//! on the filesystem for easier development, backup, and version control.

use super::{Template, TemplateError, TemplateResult, TemplateVersion};
use crate::identity::Identity;
use crate::storage::auth::AuthContext;
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{Read, Write};

/// A template registry that stores templates as files on disk
#[derive(Clone)]
pub struct FileBackedTemplateRegistry {
    /// Directory where templates are stored
    templates_dir: PathBuf,
}

impl FileBackedTemplateRegistry {
    /// Create a new file-backed template registry
    pub fn new<P: AsRef<Path>>(templates_dir: P) -> TemplateResult<Self> {
        let templates_dir = templates_dir.as_ref().to_path_buf();
        fs::create_dir_all(&templates_dir)?;
        
        Ok(Self { templates_dir })
    }
    
    /// Get the path for a specific template
    fn template_path(&self, id: &str) -> PathBuf {
        self.templates_dir.join(format!("{}.json", id))
    }
    
    /// Check if a template exists
    pub fn template_exists(&self, id: &str) -> bool {
        self.template_path(id).exists()
    }
    
    /// Create a new template
    pub fn create_template(
        &self,
        name: &str,
        mut definition: Template,
        author: &Identity,
    ) -> TemplateResult<String> {
        // Generate a unique ID if not present
        if definition.id.is_empty() {
            definition.id = format!("template:{}", uuid::Uuid::new_v4());
        }
        
        // Set version information
        let now = Utc::now().timestamp() as u64;
        let version = TemplateVersion {
            version: "1.0".to_string(),
            author: author.id().to_string(),
            created_at: now,
            description: format!("Initial version of {}", name),
        };
        
        definition.version = version;
        definition.name = name.to_string();
        
        // Serialize and write to file
        let json = serde_json::to_string_pretty(&definition)
            .map_err(|e| TemplateError::InvalidFormat { details: e.to_string() })?;
        
        let path = self.template_path(&definition.id);
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        
        Ok(definition.id.clone())
    }
    
    /// Get a template by ID
    pub fn get_template(&self, id: &str) -> TemplateResult<Template> {
        let path = self.template_path(id);
        if !path.exists() {
            return Err(TemplateError::TemplateNotFound { id: id.to_string() });
        }
        
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        serde_json::from_str(&contents)
            .map_err(|e| TemplateError::InvalidFormat { details: e.to_string() })
    }
    
    /// List all templates
    pub fn list_templates(&self) -> TemplateResult<Vec<Template>> {
        let mut templates = Vec::new();
        
        for entry in fs::read_dir(&self.templates_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            // Only process JSON files
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let mut file = File::open(path)?;
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                
                match serde_json::from_str(&contents) {
                    Ok(template) => templates.push(template),
                    Err(e) => {
                        eprintln!("Error parsing template file: {}", e);
                        // Continue with other templates, don't fail the whole operation
                    }
                }
            }
        }
        
        Ok(templates)
    }
    
    /// Update an existing template
    pub fn update_template(
        &self,
        id: &str,
        mut updated_definition: Template,
        author: &Identity,
    ) -> TemplateResult<()> {
        // Get the existing template
        let mut template = self.get_template(id)?;
        
        // Store the current version in previous versions
        template.previous_versions.push(template.version.clone());
        
        // Update version information
        let now = Utc::now().timestamp() as u64;
        let new_version = TemplateVersion {
            version: format!(
                "{}.{}",
                template.version.version.split('.').next().unwrap_or("1"),
                template.previous_versions.len() + 1
            ),
            author: author.id().to_string(),
            created_at: now,
            description: format!("Updated version of {}", template.name),
        };
        
        updated_definition.version = new_version;
        updated_definition.previous_versions = template.previous_versions;
        
        // Serialize and write to file
        let json = serde_json::to_string_pretty(&updated_definition)
            .map_err(|e| TemplateError::InvalidFormat { details: e.to_string() })?;
        
        let path = self.template_path(id);
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        
        Ok(())
    }
    
    /// Delete a template
    pub fn delete_template(&self, id: &str) -> TemplateResult<()> {
        let path = self.template_path(id);
        if !path.exists() {
            return Err(TemplateError::TemplateNotFound { id: id.to_string() });
        }
        
        fs::remove_file(path)?;
        Ok(())
    }
    
    /// Create a backup of all templates
    pub fn backup<P: AsRef<Path>>(&self, backup_dir: P) -> TemplateResult<usize> {
        let backup_dir = backup_dir.as_ref();
        fs::create_dir_all(backup_dir)?;
        
        let mut count = 0;
        for entry in fs::read_dir(&self.templates_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let filename = path.file_name().unwrap();
                let dest_path = backup_dir.join(filename);
                fs::copy(&path, &dest_path)?;
                count += 1;
            }
        }
        
        Ok(count)
    }
    
    /// Get a specific version of a template
    pub fn get_template_version(
        &self,
        id: &str,
        version: &str,
    ) -> TemplateResult<Template> {
        let template = self.get_template(id)?;
        
        // Check if it's the current version
        if template.version.version == version {
            return Ok(template);
        }
        
        // Check previous versions
        for prev_version in &template.previous_versions {
            if prev_version.version == version {
                // Here we would normally reconstruct the template at this version
                // For now, we just return a not implemented error
                return Err(TemplateError::StorageError {
                    details: "Retrieving specific versions is not yet implemented".to_string(),
                });
            }
        }
        
        Err(TemplateError::TemplateNotFound {
            id: format!("{}@{}", id, version),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;
    use std::collections::HashMap;
    use tempfile::tempdir;
    
    fn create_test_template() -> Template {
        Template {
            id: "".to_string(),
            name: "Test Template".to_string(),
            version: TemplateVersion {
                version: "1.0".to_string(),
                author: "test_author".to_string(),
                created_at: 0,
                description: "Test version".to_string(),
            },
            previous_versions: Vec::new(),
            parameters: HashMap::new(),
            voting: super::super::VotingConfig {
                quorum: 0.5,
                threshold: 0.5,
                method: super::super::VotingMethod::SimpleMajority,
                deliberation_period: 86400, // 1 day
                voting_period: 604800,      // 1 week
            },
            eligibility: super::super::EligibilityConfig {
                required_role: None,
                minimum_reputation: None,
                custom_logic: None,
            },
            execution: super::super::ExecutionConfig {
                on_approve: vec!["emit \"Proposal approved\"".to_string()],
                on_reject: None,
                execution_delay: None,
            },
        }
    }
    
    #[test]
    fn test_create_and_get_template() {
        let temp_dir = tempdir().unwrap();
        let registry = FileBackedTemplateRegistry::new(temp_dir.path()).unwrap();
        let identity = Identity::new("test_author".to_string());
        
        let template = create_test_template();
        let id = registry.create_template("Test Template", template, &identity).unwrap();
        
        let retrieved = registry.get_template(&id).unwrap();
        assert_eq!(retrieved.name, "Test Template");
        assert_eq!(retrieved.version.author, "test_author");
    }
    
    #[test]
    fn test_list_templates() {
        let temp_dir = tempdir().unwrap();
        let registry = FileBackedTemplateRegistry::new(temp_dir.path()).unwrap();
        let identity = Identity::new("test_author".to_string());
        
        // Create a few templates
        let template1 = create_test_template();
        let template2 = create_test_template();
        
        registry.create_template("Template 1", template1, &identity).unwrap();
        registry.create_template("Template 2", template2, &identity).unwrap();
        
        let templates = registry.list_templates().unwrap();
        assert_eq!(templates.len(), 2);
    }
    
    #[test]
    fn test_update_template() {
        let temp_dir = tempdir().unwrap();
        let registry = FileBackedTemplateRegistry::new(temp_dir.path()).unwrap();
        let identity = Identity::new("test_author".to_string());
        
        // Create a template
        let mut template = create_test_template();
        let id = registry.create_template("Original Name", template.clone(), &identity).unwrap();
        
        // Update the template
        template.name = "Updated Name".to_string();
        registry.update_template(&id, template, &identity).unwrap();
        
        // Verify the update
        let updated = registry.get_template(&id).unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.previous_versions.len(), 1);
    }
    
    #[test]
    fn test_delete_template() {
        let temp_dir = tempdir().unwrap();
        let registry = FileBackedTemplateRegistry::new(temp_dir.path()).unwrap();
        let identity = Identity::new("test_author".to_string());
        
        // Create a template
        let template = create_test_template();
        let id = registry.create_template("Test Template", template, &identity).unwrap();
        
        // Verify it exists
        assert!(registry.template_exists(&id));
        
        // Delete it
        registry.delete_template(&id).unwrap();
        
        // Verify it's gone
        assert!(!registry.template_exists(&id));
    }
} 