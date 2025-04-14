use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

// --- Configuration ---
const GIT_REPO_URL: &str = "https://github.com/PrismarineJS/minecraft-data.git";
const GIT_BRANCH: &str = "master"; // Use master branch

// Location within OUT_DIR to clone the repo
const CLONE_DEST_SUBDIR: &str = "minecraft-data-repo-clone";
// Location within the clone where the data resides
const CLONED_DATA_SUBDIR: &str = "data";
// Final location within the crate's source tree for vendored data
const VENDORED_DATA_DIR: &str = "src/minecraft_data_vendored";
// --- End Configuration ---


fn main() {
    // Rerun if build.rs changes OR if the cloned repo's HEAD changes
    // This is less reliable than tracking specific files but attempts to rebuild on repo update.
    // A more robust way would involve checking the remote HEAD, which is complex in build.rs.
    println!("cargo:rerun-if-changed=build.rs");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let clone_head_path = Path::new(&manifest_dir)
        .join("target") // Assume clone is within target via OUT_DIR
        .join(env::var("PROFILE").unwrap_or_else(|_| "debug".to_string()))
        .join("build")
        // Finding the exact build script dir hash is tricky, so we make a best guess
        // This rerun trigger might not always work perfectly on branch changes.
        .join(format!("{}-*", env::var("CARGO_PKG_NAME").unwrap_or_default()))
        .join("out")
        .join(CLONE_DEST_SUBDIR)
        .join(".git/refs/heads")
        .join(GIT_BRANCH);
    if clone_head_path.exists() {
         println!("cargo:rerun-if-changed={}", clone_head_path.display());
    }


    let out_dir = env::var("OUT_DIR").expect("OUT_DIR environment variable not set");

    let clone_dest = Path::new(&out_dir).join(CLONE_DEST_SUBDIR);
    let cloned_data_path = clone_dest.join(CLONED_DATA_SUBDIR);
    let vendored_path = Path::new(&manifest_dir).join(VENDORED_DATA_DIR);

    // 1. Clone or update the repository in OUT_DIR
    if !clone_dest.exists() {
        eprintln!(
            "Cloning branch '{}' from {} into {}...",
            GIT_BRANCH,
            GIT_REPO_URL,
            clone_dest.display()
        );
        if !run_command(
            "git",
            &["clone", "--depth", "1", "--branch", GIT_BRANCH, "--single-branch", GIT_REPO_URL, CLONE_DEST_SUBDIR],
            &out_dir,
            "Failed to clone repository",
        ) {
            panic!("Git clone failed.");
        }
    } else {
        eprintln!("Repository already cloned in {}. Pulling latest changes from '{}'...", clone_dest.display(), GIT_BRANCH);
        // Pull the latest changes from the specified branch
        if !run_command(
            "git",
            &["-C", CLONE_DEST_SUBDIR, "checkout", GIT_BRANCH], // Ensure we are on the right branch first
            &out_dir,
            "Failed to checkout branch",
        ) || !run_command(
            "git",
            &["-C", CLONE_DEST_SUBDIR, "pull", "origin", GIT_BRANCH], // Pull latest
            &out_dir,
            "Failed to pull latest changes",
        ) {
             eprintln!("Warning: Failed to pull latest changes from git repository. Using cached version.");
             // Proceed with cached version, but warn the user.
        }
    }

    // 2. Check if the data directory exists within the clone
    if !cloned_data_path.exists() {
        panic!(
            "Cloned repository at '{}' is missing the data directory '{}'.",
            clone_dest.display(), CLONED_DATA_SUBDIR
        );
    }

    // 3. Copy data from clone to vendored location in src/
    eprintln!(
        "Vendoring data from '{}' to '{}'...",
        cloned_data_path.display(),
        vendored_path.display()
    );

    // Remove old vendored data first
    if vendored_path.exists() {
        fs::remove_dir_all(&vendored_path)
            .unwrap_or_else(|e| panic!("Failed to remove old vendored data directory '{}': {}", vendored_path.display(), e));
    }

    // Set up copy options
    let mut options = fs_extra::dir::CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = false; // Copy the 'data' folder itself into src/

    // Perform the copy into src/ (parent of VENDORED_DATA_DIR)
    match fs_extra::dir::copy(&cloned_data_path, &vendored_path.parent().unwrap(), &options) {
        Ok(_) => eprintln!("Successfully copied data directory."),
        Err(e) => panic!(
            "Failed to copy data from '{}' to '{}': {}",
            cloned_data_path.display(),
            vendored_path.parent().unwrap().display(),
            e
        ),
    }

    // Rename the copied 'data' folder to 'minecraft_data_vendored' inside src/
    let copied_data_path_in_src = vendored_path.parent().unwrap().join(CLONED_DATA_SUBDIR);
    if copied_data_path_in_src.exists() {
        fs::rename(&copied_data_path_in_src, &vendored_path)
            .unwrap_or_else(|e| panic!("Failed to rename copied data directory '{}' to '{}': {}",
                copied_data_path_in_src.display(), vendored_path.display(), e));
        eprintln!("Successfully renamed vendored data directory.");
    } else {
         panic!("Copied data directory '{}' not found after copy operation.", copied_data_path_in_src.display());
    }


    // 4. Generate the path constant file in OUT_DIR
    let dest_path = Path::new(&out_dir).join("vendored_data_path.rs");
    fs::write(
        &dest_path,
        format!(
            "/// Path to the vendored minecraft-data relative to the crate root.\n\
             pub const VENDORED_MINECRAFT_DATA_PATH: &str = \"{}\";",
            VENDORED_DATA_DIR // Use the relative path for inclusion in source
        ),
    )
    .expect("Failed to write vendored_data_path.rs");

    eprintln!("Build script finished.");
}

// Helper function to run a command
fn run_command(program: &str, args: &[&str], current_dir: &str, failure_msg: &str) -> bool {
    eprintln!("Running command: {} {:?}", program, args);
    let status_result = Command::new(program)
        .args(args)
        .current_dir(current_dir)
        .stdout(Stdio::inherit()) // Show command output
        .stderr(Stdio::inherit()) // Show command errors
        .status();

    match status_result {
        Ok(status) => {
            if !status.success() {
                eprintln!("{}: Command exited with status: {}", failure_msg, status);
            }
            status.success()
        }
        Err(e) => {
            eprintln!("{}: Failed to execute command '{} {:?}': {}", failure_msg, program, args, e);
            false
        }
    }
}