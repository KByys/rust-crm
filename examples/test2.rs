use crm_rust::Config;
fn main() {
    let setting = Config::read();
    println!("{:#?}", setting);
}
