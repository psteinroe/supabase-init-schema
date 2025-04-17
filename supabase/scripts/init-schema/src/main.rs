use parse::get_nodes;
use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use write::write_nodes;

mod locations;
mod parse;
mod write;

fn find_supabase_dir() -> PathBuf {
    let mut current_dir = env::current_dir().expect("Failed to get current directory");

    // Keep going up until we find the Supabase directory that contains config.toml
    loop {
        // Check if this is the supabase directory with config.toml
        if current_dir
            .file_name()
            .is_some_and(|name| name == "supabase")
            && current_dir.join("config.toml").exists()
        {
            return current_dir;
        }

        // Go up one directory
        if !current_dir.pop() {
            panic!("Could not find Supabase root directory (with config.toml)");
        }
    }
}

fn main() {
    // Find the Supabase root directory
    let supabase_dir = find_supabase_dir();
    println!("Found Supabase directory at: {}", supabase_dir.display());

    let status = Command::new("supabase")
        .args(["status"])
        .current_dir(&supabase_dir)
        .status()
        .expect("Failed to reset database");

    // For some reason, there is no start --no-seed so we have to start first and then reset...
    if !status.success() {
        println!("Supabase is not running. Starting Supabase...");
        let status = Command::new("supabase")
            .args(["start"])
            .current_dir(&supabase_dir)
            .status()
            .expect("Failed to reset database");

        if !status.success() {
            eprintln!("Failed to start Supabase");
            return;
        }
    }

    // Reset the database without seeding
    println!("Resetting Supabase database without seeding...");
    let reset_status = Command::new("supabase")
        .args(["db", "reset", "--no-seed"])
        .current_dir(&supabase_dir)
        .status()
        .expect("Failed to reset database");

    if !reset_status.success() {
        eprintln!("Database reset failed");
        return;
    }

    // Dump the schema directly to memory
    println!("Dumping schema...");
    let dump_output = Command::new("supabase")
        .args([
            "db",
            "dump",
            "--local",
            "-s",
            "public,private,api,async_trigger",
        ])
        .current_dir(&supabase_dir)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start schema dump")
        .stdout
        .expect("Failed to capture stdout");

    // Read the output into a string
    let mut schema = String::new();
    let mut dump_reader = std::io::BufReader::new(dump_output);
    dump_reader
        .read_to_string(&mut schema)
        .expect("Failed to read schema dump output");

    // Process the schema
    println!("Processing schema...");
    let nodes = get_nodes(&schema);

    let out_dir = supabase_dir.join("schemas");

    // remove the existing schemas directory if it exists
    let _ = fs::remove_dir_all(&out_dir);

    write_nodes(&nodes, &out_dir);

    println!("Schema initialization completed successfully!");
}
