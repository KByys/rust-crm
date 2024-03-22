use mysql::params;
use serde_json::json;

macro_rules! tes {
    ($name:expr, $de:ident) => {
        stringify!($de)
    };
}

fn main() {
    let d = tes!(1, NAME);
    println!("{}", d)
}
