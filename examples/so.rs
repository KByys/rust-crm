use mysql::params;
use serde_json::json;

macro_rules! tes {
    ($name:expr, $de:ident) => {
        stringify!($de)
    };
}

fn main() {
    for i in 0..100 {
        for j in 0..100 {
            let f1 = i as f32 * 0.01;
            let f2 = j as f32 * 0.01;
            let f3 = (i + j) as f32 * 0.01;
            println!("{i} {j} --- {}  {}", f1 + f2, f3);
            assert_eq!(f1 + f2, f3)
        }
    }
}
