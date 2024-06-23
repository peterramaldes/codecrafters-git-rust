use clap::{Parser, Subcommand};
use core::panic;
use flate2::read::ZlibDecoder;
use std::{
    fs::{self, File},
    io::Read,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a git repository
    Init,

    /// Provide content or type and size information for repository objects
    CatFile {
        /// Pretty-print the contents of <object> based on its type
        #[arg(short, value_name = "object")]
        p: String,
    },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Init => init(),
        Commands::CatFile { p } => cat_file(p),
    }
}

fn init() {
    fs::create_dir(".git").unwrap();
    fs::create_dir(".git/objects").unwrap();
    fs::create_dir(".git/refs").unwrap();
    fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
    println!("Initialized git directory")
}

enum ObjectType {
    Blob,
}

struct Object {
    _object_type: ObjectType,
    _byte_size: usize,
    content: String,
}

fn cat_file(object: &String) {
    let filepath = format!(".git/objects/{}/{}", &object[..2], &object[2..]);
    let compressed_file = File::open(filepath).expect("expected a file but it was not found");
    let mut decompressed_data = Vec::new();

    ZlibDecoder::new(compressed_file)
        .read_to_end(&mut decompressed_data)
        .expect("expected decompressed data");

    let obj = String::from_utf8(decompressed_data)
        .map(|d| {
            let null_byte_idx = d.find('\0').unwrap();
            let object_type_and_byte_size_str = &d[..null_byte_idx];
            let mut object_type_and_byte_size_itr =
                object_type_and_byte_size_str.split_whitespace();

            let object_type = match object_type_and_byte_size_itr.next().unwrap() {
                "blob" => ObjectType::Blob,
                _ => panic!("unkown object type"),
            };

            let byte_size = object_type_and_byte_size_itr
                .next()
                .unwrap_or("0")
                .parse::<usize>()
                .unwrap_or(0);

            let content = &d[null_byte_idx + 1..];
            let content: String = content.to_string().clone();

            return Object {
                _object_type: object_type,
                _byte_size: byte_size,
                content,
            };
        })
        .expect("Error parsing decompressed data into Object struct");

    print!("{}", obj.content);
}
