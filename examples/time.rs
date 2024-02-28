use regex::Regex;

fn main() {
    let re = Regex::new(r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2})").unwrap();
    let hay = "On 2010-03-14 12:34, foo happened. On 2014-10-14, bar happened.";

    let d: Option<(&str, [&str; 5])> = re.captures(hay).map(|e| e.extract());
    println!("{:?}", d);
    // for (_, [year, month, day, hours,  min]) in re.captures_iter(hay).map(|c| c.extract()) {
    //     dates.push((year, month, day, hours, min));
    // }
    // println!("{:#?}", dates)
}
