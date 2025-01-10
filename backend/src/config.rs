use bcrypt::hash;
use once_cell::sync::Lazy;

#[derive(Clone)]
pub struct Config {
    pub jwt_secret: String,
    pub admin_credentials: AdminCredentials,
}

#[derive(Clone)]
pub struct AdminCredentials {
    pub username: String,
    pub password_hash: String,
}

impl Config {
    pub fn load() -> Self {
        dotenv::dotenv().ok();

        // Get environment
        let environment = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());

        let jwt_secret =
            std::env::var("JWT_SECRET").expect("JWT_SECRET must be set in the environment");

        let username =
            std::env::var("ADMIN_USERNAME").expect("ADMIN_USERNAME must be set in the environment");

        let password =
            std::env::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD must be set in the environment");

        let bcrypt_cost = match environment.as_str() {
            "production" => 12,
            "development" => 4,
            _ => 10,
        };

        let password_hash = hash(&password, bcrypt_cost).expect("Failed to hash password");

        Config {
            jwt_secret,
            admin_credentials: AdminCredentials {
                username,
                password_hash,
            },
        }
    }
}

// Store config in a static variable
pub static CONFIG: Lazy<Config> = Lazy::new(Config::load);
