use regex::Regex;



static YYYY_MM_DD: &str = r"(\d{4})-(\d{2})-(\d{2})";

fn main() {
    let d = "2002-23-34 34:34:34";
    let rex = Regex::new(YYYY_MM_DD).unwrap();
    println!("{:?}", rex.captures(d).unwrap().get(0).unwrap().as_str())
}
