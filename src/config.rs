use dotenv::dotenv;
use std::env;
use once_cell::sync::Lazy;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub cors_allowed_origins: Vec<String>,
    pub auth_secret: String,
    pub auth_token_expires_in: Duration,
    pub environment: Environment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Environment {
    Development,
    Testing,
    Production,
}

impl Environment {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "production" => Self::Production,
            "testing" => Self::Testing,
            _ => Self::Development,
        }
    }
    
    pub fn is_dev(&self) -> bool {
        *self == Environment::Development
    }
    
    pub fn is_prod(&self) -> bool {
        *self == Environment::Production
    }
    
    pub fn is_test(&self) -> bool {
        *self == Environment::Testing
    }
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    dotenv().ok();
    
    let env_name = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    let environment = Environment::from_str(&env_name);
    
    Config {
        database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        server_port: env::var("SERVER_PORT")
            .unwrap_or_else(|_| "3030".to_string())
            .parse()
            .expect("SERVER_PORT must be a valid number"),
        cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect(),
        auth_secret: env::var("AUTH_SECRET").expect("AUTH_SECRET must be set"),
        auth_token_expires_in: Duration::from_secs(
            env::var("AUTH_TOKEN_EXPIRES_IN")
                .unwrap_or_else(|_| "86400".to_string()) // 24 hours
                .parse()
                .expect("AUTH_TOKEN_EXPIRES_IN must be a valid number"),
        ),
        environment,
    }
});

pub fn init() {
    Lazy::force(&CONFIG);
} 