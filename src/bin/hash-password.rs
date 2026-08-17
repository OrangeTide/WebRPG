//! Print an Argon2 hash for a password, in the format the users table stores.
//!
//! Used by `scripts/admin-reset-password.sh`. Hashing lives here rather than in
//! the script so that a reset always produces exactly what the login path
//! verifies: the same crate, the same parameters, the same PHC string format.
//!
//! PUBLIC DOMAIN (CC0-1.0)

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(password) = args.next() else {
        eprintln!("usage: hash-password <password>");
        eprintln!("Prints an Argon2 PHC string on stdout.");
        std::process::exit(2);
    };

    match webrpg::auth::hash_password(&password) {
        Ok(hash) => println!("{hash}"),
        Err(e) => {
            eprintln!("hash-password: {e}");
            std::process::exit(1);
        }
    }
}
