use std::time::Duration;



#[tokio::main]
async fn main() {
    let test1 = Test {
        name: "456456".into(),
        ..Default::default()
    };

    let test = Test {
        name: "456456".into(),
        ..test1
    };

    tokio::join!(print(0, 10), print(20, 30));

}
// #[forbid(unused)]
#[derive(Default)]
struct Test {
    name: String,
    de: String
}
async fn print(s: usize, e: usize) {
    
    for e in s..=e {
        println!("{}", e);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

}