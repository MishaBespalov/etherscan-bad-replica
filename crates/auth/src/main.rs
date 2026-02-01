use axum::handler;
use axum::serve;

fn main() {
    println!("Hello, world!");
}

// 6. Start with the basics
//
// Begin by:
// - Setting up database connection pools (sqlx::PgPool, redis::Client)
// - Creating a basic Axum server with health check endpoint
// - Implementing user registration (POST /signup)
// - Implementing login (POST /login) with JWT generation
