use bufstream::BufStream;
use smac::*;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, IpAddr};
use std::path::Path;
use std::str::FromStr;
use std::writeln;

const SP_IP_ADDR: &'static str = "127.0.0.1";
const SP_PORT: u16 = 7778;
const INPUT_IND_FILE: &'static str = "test_data/large_input_ind.txt";
const INPUT_DAT_FILE: &'static str = "test_data/large_input_dat.txt";
const OUTPUT_FILE: &'static str = "output.txt";

fn exit_print(name: &str) {
    eprintln!(
        "Usage: {} <service provider ip addr> <index input file> <data input file> <output file>",
        name
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut sp_ip_addr = SP_IP_ADDR;
    let mut ind_file = INPUT_IND_FILE;
    let mut dat_file = INPUT_DAT_FILE;
    let mut output_file = OUTPUT_FILE;
    if args.len() == 1 {
        eprintln!("Client: Using default parameters: ");
    } else if args.len() == 2 && args[1].as_str() == "-h" {
        return exit_print(&args[0]);
    } else if args.len() != 5 {
        return exit_print(&args[0]);
    } else {
        eprintln!("Client: Using command line parameters: ");
        sp_ip_addr = args[1].as_str();
        ind_file = args[2].as_str();
        dat_file = args[3].as_str();
        output_file = args[4].as_str();
    }

    eprintln!("\tService Provider IP address:\t{}", sp_ip_addr);
    eprintln!("\tService Provider port:\t{}", SP_PORT);
    eprintln!("\tInput index file:\t{}", ind_file);
    eprintln!("\tInput data file:\t{}", dat_file);
    eprintln!("\tOutput file:\t\t{}", output_file);

    let mut stream = BufStream::new(tcp_keep_connecting(
        SocketAddr::from((IpAddr::from_str(sp_ip_addr).unwrap(), SP_PORT)),
    ));

    eprintln!("Client: connected to Server");

    eprintln!("Client: start sending inputs");

    let n_ind = BufReader::new(File::open(ind_file).unwrap())
        .lines()
        .count();

    let mut input_writer = InputWriter::new(n_ind, &Path::new(ind_file), &Path::new(dat_file));
    input_writer.stream(&mut stream).unwrap();
    stream.flush().unwrap();

    eprintln!("Client: done sending inputs");

    let imputed = StreamOutputReader::read(stream).collect::<Vec<Real>>();

    let mut file = File::create(output_file).unwrap();
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
