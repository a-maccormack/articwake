use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: hash_pin <PIN>");
        std::process::exit(1);
    }

    let pin = &args[1];
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(pin.as_bytes(), &salt)
        .expect("Failed to hash PIN");

    println!("{}", hash);
}
