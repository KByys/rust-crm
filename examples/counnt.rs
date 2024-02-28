use std::{collections::VecDeque, fs::read_dir, path::PathBuf};

fn main() -> std::io::Result<()> {
    let mut queue = VecDeque::new();
    queue.push_back(PathBuf::from("src"));
    let mut count = 0;
    while let Some(path) = queue.pop_front() {
        for dir in read_dir(path)? {
            let path = dir.unwrap().path();
            if path.is_dir() {
                queue.push_back(path)
            } else {
                let data = std::fs::read_to_string(path)?;
                let d: Vec<_> = data.lines().collect();
                count += d.len();
            }
        }
    }
    println!("{}", count);
    Ok(())
}
