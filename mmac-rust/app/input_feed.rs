use mmac::*;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
const INPUT_IND_FILE: &'static str = "input_ind.txt";
const INPUT_DAT_FILE: &'static str = "input_dat.txt";

fn main() {
    eprintln!(
        "InputFeed: loading from input {} and {}",
        INPUT_IND_FILE, INPUT_DAT_FILE
    );
    let input_ind_path = Path::new(INPUT_IND_FILE);
    let input_dat_path = Path::new(INPUT_DAT_FILE);

    let stream = tcp_keep_connecting(SocketAddr::from_str("127.0.0.1:7778").unwrap());

    eprintln!("InputFeed: connected to Main");
    eprintln!("InputFeed: start feeding...");

    let mut input_writer = InputWriter::new(&input_ind_path, &input_dat_path);
    input_writer.write(stream).unwrap();

    eprintln!("InputFeed: done");
}
