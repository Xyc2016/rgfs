use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
/// Transform staged files using a formatting command that accepts content via stdin and produces a result via stdout.
#[command(version, about)]
struct Cli {
    /// Shell command to format files, will run once per file. Occurrences of the placeholder `{}` will be replaced with a path to the file being formatted. (Example: "prettier --stdin-filepath '{}'")
    #[arg(short, long)]
    formatter: String,

    /// By default formatting changes made to staged file content will also be applied to working tree files via a patch. This option disables that behavior, leaving working tree files untouched.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    no_update_working_tree: bool,

    /// Prevents rgfs from modifying staged or working tree files. You can use this option to check staged changes with a linter instead of formatting. With this option stdout from the formatter command is ignored. Example: rgfs --no-write -f "eslint --stdin --stdin-filename '{}' >&2" "*.js"
    #[arg(long, action = clap::ArgAction::SetTrue)]
    no_write: bool,

    /// Show the formatting commands that are running
    #[arg(long, action = clap::ArgAction::SetTrue)]
    verbose: bool,

    /// Patterns that specify files to format. The formatter will only transform staged files that are given here. Patterns may be literal file paths, or globs which will be tested against staged file paths using Python's fnmatch function. For example "src/*.js" will match all files with a .js extension in src/ and its subdirectories. Patterns may be negated to exclude files using a "!" character. Patterns are evaluated left-to-right. (Example: "main.js" "src/*.js" "test/*.js" "!test/todo/*")
    #[arg(action = clap::ArgAction::Append)]
    files: Vec<PathBuf>,
}

fn get_git_root() -> PathBuf {
    let output = process::Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .expect("Failed to run git rev-parse --show-toplevel");
    let git_root = std::str::from_utf8(&output.stdout)
        .expect("Failed to parse git rev-parse --show-toplevel output")
        .trim();
    PathBuf::from(git_root)
}

fn main() {
    let cli = Cli::parse();
    dbg!(cli);
}
