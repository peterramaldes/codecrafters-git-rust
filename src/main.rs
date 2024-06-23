use clap::{Parser, Subcommand};
use core::{fmt, panic};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};
use std::{
    fs::{self, metadata, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
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

    /// Compute object ID and optionally creates a blob from a file
    HashObject {
        /// Actually write the object into the object database.
        #[arg(short, value_name = "object")]
        w: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Init => init(),
        Commands::CatFile { p } => cat_file(p),
        Commands::HashObject { w } => hash_object(w),
    }
}

fn init() {
    fs::create_dir(".git").unwrap();
    fs::create_dir(".git/objects").unwrap();
    fs::create_dir(".git/refs").unwrap();
    fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
    println!("Initialized git directory")
}

#[derive(Debug)]
enum ObjectType {
    Blob,
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectType::Blob => write!(f, "blob"),
        }
    }
}

#[derive(Debug)]
struct Object {
    object_type: ObjectType,
    byte_size: u64,
    content: String,
}

impl Object {
    fn file_format(&self) -> String {
        let obj_type = &self.object_type;
        let byte_size = &self.byte_size;
        let content = &self.content;
        return format!("{obj_type} {byte_size}\0{content}");
    }

    fn compress_and_write(&self) {
        let hash = self.hash();

        let dir = format!(".git/objects/{}", &hash[..2]);
        fs::create_dir(&dir).expect(&format!("cannot create the dir: {}", &dir));

        let file_path = format!("{dir}/{}", &hash[2..]);
        let output_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)
            .unwrap();

        let mut encoder = ZlibEncoder::new(output_file, Compression::default());

        encoder.write_all(&self.file_format().as_bytes()).unwrap();
        encoder.finish().unwrap();
    }

    fn hash(&self) -> String {
        let file = self.file_format();
        let mut hasher = Sha1::new();
        hasher.update(file.as_bytes());
        return format!("{:x}", hasher.finalize());
    }
}

// TODO: Move this code to be a implementation of Object
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

            let t = object_type_and_byte_size_itr.next().unwrap();
            let object_type = match t {
                "blob" => ObjectType::Blob,
                _ => panic!("unsupported type: {t}"), // TODO: if we try to `cat-file` a tree for instance, the code will not reach this panic, but will throw error on `expect` function.
            };

            let byte_size = object_type_and_byte_size_itr
                .next()
                .unwrap_or("0")
                .parse::<u64>()
                .unwrap_or(0);

            let content = &d[null_byte_idx + 1..];
            let content: String = content.to_string().clone();

            return Object {
                object_type,
                byte_size,
                content,
            };
        })
        .expect("Error parsing decompressed data into Object struct");

    print!("{}", obj.content);
}

// TODO: Move this code to be a implementation of Object
fn hash_object(file_path: &PathBuf) {
    let bytes = metadata(&file_path)
        .expect("error to get metadata from file path {file_path}")
        .len();

    let content = fs::read_to_string(&file_path).expect(&format!(
        "error to get content from the file {:?}",
        file_path
    ));

    let obj: Object = Object {
        object_type: ObjectType::Blob,
        byte_size: bytes,
        content: content,
    };

    obj.compress_and_write();

    println!("{}", obj.hash())
}
