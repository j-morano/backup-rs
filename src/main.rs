use std::fs;
use std::path::Path;



/// Get the size of a file
fn size(file: &str) -> u64 {
    let file = fs::File::open(file).unwrap();
    let metadata = file.metadata().unwrap();
    metadata.len()
}


/// Get the last modified time of a file
fn modified_time(file: &str) -> std::time::SystemTime {
    let metadata = fs::metadata(file).unwrap();
    metadata.modified().unwrap()
}


/// Check if a file is a symlink
fn is_symlink(file: &str) -> i32 {
    match fs::symlink_metadata(file) {
        Ok(metadata) => if metadata.file_type().is_symlink() {
            return 0;
        } else {
            return 1;
        },
        Err(_) => return 2,
    }
}


/// Recursively iterate through the destination directory to remove the files
/// that are not in the source directory
fn remove_removed(source: &str, destination: &str, dry_run: bool) {
    for entry in fs::read_dir(destination).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            // Recursively call remove_removed() for subdirectories
            // If the subdirectory doesn't exist in the source directory,
            // remove it from the destination directory
            let subdirectory = path.file_name().unwrap().to_str().unwrap();
            let source = format!("{}/{}", source, subdirectory);
            if !Path::new(&source).exists() {
                println!("Removing directory: {}", path.to_str().unwrap());
                if !dry_run {
                    fs::remove_dir_all(path).unwrap();
                }
            } else {
                remove_removed(&source, path.to_str().unwrap(), dry_run);
            }
        } else {
            // If the file doesn't exist in the source directory,
            // remove it from the destination directory
            let file_name = path.file_name().unwrap();
            let file_name_str = match file_name.to_str() {
                Some(s) => s,
                None => continue,
            };
            let source_file = format!("{}/{}", source, file_name_str);
            if is_symlink(path.to_str().unwrap()) == 0 {
                match fs::read_link(source_file) {
                    Ok(_) => (),
                    Err(_) => {
                        println!("Removing symlink: {}", path.to_str().unwrap());
                        if !dry_run {
                            fs::remove_dir_all(path.clone()).unwrap();
                        }
                    }
                }
            } else {
                if !Path::new(&source_file).exists() {
                    println!("Removing file: {}", path.to_str().unwrap());
                    if !dry_run {
                        fs::remove_file(path).unwrap();
                    }
                }
            }
        }
    }
}


fn copy_file(source: &str, destination: &str, dry_run: bool) {
    println!("Copying {} to {}", source, destination);
    if !dry_run {
        if is_symlink(source) == 0 {
            // Create a symlink in the destination directory
            // pointing to the source file
            // This is a workaround for the fs::copy() function
            // not working with symlinks
            let source = fs::read_link(source).unwrap();
            std::os::unix::fs::symlink(source, destination.clone()).unwrap();
        } else {
            fs::copy(source, destination).unwrap();
        }
    }
}


/// Backup the source directory to the destination directory
fn backup(source: &str, destination: &str, dry_run: bool) {
    // Get a list (recursively) of the files in the source directory
    // and copy them to the destination directory, preserving the
    // directory structure
    let dir = match fs::read_dir(source) {
        Ok(d) => d,
        Err(_) => {
            return;
        }
    };
    for entry in dir {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            // Recursively call backup() for subdirectories
            // Create the subdirectory in the destination directory
            // if it doesn't exist
            let subdirectory = path.file_name().unwrap().to_str().unwrap();
            let destination = format!("{}/{}", destination, subdirectory);
            if !Path::new(&destination).exists() {
                if !dry_run {
                    fs::create_dir(&destination).unwrap();
                }
            }
            backup(path.to_str().unwrap(), &destination, dry_run);
        } else {
            // Copy the file to the destination directory
            let file_name = path.file_name().unwrap();
            let file_name_str = match file_name.to_str() {
                Some(s) => s,
                None => continue,
            };
            let destination_file = format!("{}/{}", destination, file_name_str);
            let source_file = path.to_str().unwrap();
            if is_symlink(source_file) == 0 {
                if is_symlink(&destination_file) == 0 {
                    // If the symlink in the source directory points to a different
                    // file than the symlink in the destination directory, overwrite
                    // the destination symlink
                    let source = fs::read_link(source_file).unwrap();
                    let destination = fs::read_link(&destination_file).unwrap();
                    if source != destination {
                        copy_file(source_file, &destination_file, dry_run);
                    }
                } else {
                    // If the destination file is not a symlink, overwrite it
                    copy_file(source_file, &destination_file, dry_run);
                }
            } else if Path::new(&destination_file).exists() {
                // Get size of both files, and if they are different, overwrite
                // the destination file
                if size(source_file) != size(&destination_file) {
                    copy_file(source_file, &destination_file, dry_run);
                } else {
                    if modified_time(source_file) > modified_time(&destination_file) {
                        copy_file(source_file, &destination_file, dry_run);
                    }
                }
            } else {
                copy_file(source_file, &destination_file, dry_run);
            }
        }
    }
}


fn print_usage_and_exit(code: i32) {
    const USAGE: &str = "\
    Usage: backup-rs [OPTION]... SOURCE DESTINATION

    OPTIONS:
      --dry  simulate the backup process
      --help  display this help and exit
      --version  output version information and exit

    Exit status:
      0  if OK,
      1  if minor problems (e.g., cannot access subdirectory)

    Full documentation <https://github.com/j-morano/contemporary-z>
    ";
    println!("{}", USAGE);
    std::process::exit(code);
}



fn main() {
    // Process command line arguments
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() == 2 {
        if args[1] == "--help" {
            print_usage_and_exit(0);
        } else if args[1] == "--version" {
            // Print the version of the program from the Cargo.toml file
            let version = env!("CARGO_PKG_VERSION");
            println!("backup-rs {}", version);
            std::process::exit(0);
        } else {
            print_usage_and_exit(1);
        }
    } else if args.len() == 3 || args.len() == 4 {
        let mut dry_run = false;
        if args.len() == 4 {
            let mut i = 0;
            let mut loc = 0;
            for arg in &args {
                if arg == "--dry" {
                    dry_run = true;
                    loc = i;
                }
                i += 1;
            }
            if !dry_run {
                print_usage_and_exit(1);
            } else {
                args.remove(loc);
            }
        }
        let source = &args[1];
        let destination = &args[2];
        println!("{}", "-".repeat(80));
        println!("Source: {}", source);
        println!("Destination: {}", destination);
        println!("{}", "-".repeat(80));

        if !dry_run {
            println!("Backup in progress...");
        } else {
            println!("Dry run: Backup simulation in progress...");
        }
        if !dry_run {
            // Create the destination directory if it doesn't exist
            if !Path::new(destination).exists() {
                fs::create_dir(destination).unwrap();
            }
        }

        // Recursively iterate through the destination directory to remove the files
        // that are not in the source directory
        remove_removed(source, destination, dry_run);

        println!("{}", "-".repeat(80));
        // Backup the source to the destination
        backup(&args[1], &args[2], dry_run);
    } else {
        print_usage_and_exit(1);
    }
}
