//! Process failure fixture compiled only by integration tests, never packaged.
use std::io::{Read,Write};
fn main() {
    let path=std::env::current_exe().unwrap();
    let name=path.file_stem().unwrap().to_str().unwrap();
    let mut input=Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();
    if name.starts_with("crash") {std::process::exit(17);}
    if name.starts_with("invalid") {print!("{{\"unexpected\":true}}");return;}
    if name.starts_with("oversized") {
        let _=std::io::stdout().write_all(&vec![b'x';1024*1024]);
    }
    loop {std::thread::sleep(std::time::Duration::from_millis(100));}
}
