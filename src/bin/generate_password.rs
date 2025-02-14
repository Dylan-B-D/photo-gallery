use argon2::{Argon2, PasswordHasher, password_hash::{SaltString, rand_core::OsRng}};
use std::io::{self, Write};

fn main() {
    // Prompt the user for a password
    print!("Enter your password: ");
    io::stdout().flush().unwrap();

    let mut password = String::new();
    io::stdin()
        .read_line(&mut password)
        .expect("Failed to read input");

    // Trim the newline character from the input
    let password = password.trim();

    // Generate a random salt
    let salt = SaltString::generate(&mut OsRng);

    // Hash the password using Argon2
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();

    // Escape the `$` characters in the hash for use in .env files
    let escaped_hash = password_hash.to_string().replace('$', "\\$");

    // Output the hashed password
    println!("Hashed password: {}", escaped_hash);
}