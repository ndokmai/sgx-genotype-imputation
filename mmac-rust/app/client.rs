use mmac::*;
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use std::writeln;
const INPUT_IND_FILE: &'static str = "input_ind.txt";
const INPUT_DAT_FILE: &'static str = "input_dat.txt";
const OUTPUT_FILE: &'static str = "output.txt";

fn main() {
    eprintln!(
        "Client: loading from input {} and {}",
        INPUT_IND_FILE, INPUT_DAT_FILE
    );

    let input_ind_path = Path::new(INPUT_IND_FILE);
    let input_dat_path = Path::new(INPUT_DAT_FILE);

    let mut stream = tcp_keep_connecting(SocketAddr::from_str("127.0.0.1:7778").unwrap());

    eprintln!("Client: connected to Server");

    eprintln!("Client: start sending inputs");

    let n_ind = 97020;

    let mut input_writer = InputWriter::new(n_ind, &input_ind_path, &input_dat_path);
    input_writer.write(&mut stream).unwrap();

    let imputed = StreamOutputReader::read(stream).collect::<Vec<Real>>();

    let mut file = File::create(OUTPUT_FILE).unwrap();
    writeln!(
        file,
        "{}",
        imputed
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join("\n")
    )
    .unwrap();

    eprintln!("Client: imputation result written to {}", OUTPUT_FILE);

    eprintln!("Client: done");
}
