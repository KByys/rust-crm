
fn main() {
    let dt = "123456";
    println!("{:?}", dt.splitn(2, '-').collect::<Vec<&str>>())
}