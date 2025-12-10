use std::path::Path;

fn main() {
    println!("/foo is_absolute: {}", Path::new("/foo").is_absolute());
    println!("C:\\foo is_absolute: {}", Path::new("C:\\foo").is_absolute());
    println!("\\\\server\\share is_absolute: {}", Path::new("\\\\server\\share").is_absolute());
}
