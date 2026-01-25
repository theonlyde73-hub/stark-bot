use std::env;

#[derive(Clone)]
pub struct Config {
    pub secret_key: String,
    pub port: u16,
    pub gateway_port: u16,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            secret_key: env::var("SECRET_KEY").expect("SECRET_KEY must be set"),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a valid number"),
            gateway_port: env::var("GATEWAY_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()
                .expect("GATEWAY_PORT must be a valid number"),
            database_url: env::var("DATABASE_URL").unwrap_or_else(|_| "./.db/stark.db".to_string()),
        }
    }
}
