use std::env;
fn main() {
    if let Ok(flag) = env::var("FLAG") {
        println!("{}", flag);
    }
}
