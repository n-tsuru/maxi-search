mod create_files;
mod expand_files;
mod query;
mod index;

use create_files::create_files;
use expand_files::expand_file;

#[allow(unused_imports)]
use log::{info, warn, Level};
use simple_logger::SimpleLogger;

// args
use clap::{arg, command, value_parser};
use std::os::fd::AsRawFd;
use std::path::PathBuf;

// file operation for create index
use std::fs;

fn main() -> std::io::Result<()> {
    let matches = command!()
        .subcommand_required(true)
        .author("Nobuhiko Tsuruoka, takanotume@gmail.com")
        .version("0.1")
        .about("create compressed index and search")
        .arg(arg!(-i --index <INDEX>)
            .help("index file")
            .value_parser(value_parser!(PathBuf))
            .default_value("index.dat"))
        .arg(arg!(-c --chunk <CHUNK>)
            .help("chunk size should be 2^N")
            .value_parser(["4M","8M","16M"])
            .default_value("4M"))
        .arg(arg!(-l --log <LOG>)
            .default_value("info")
            .value_parser(["debug","info","warn"])
            .help("Set the logging level. Options: [error, warn, info, debug, trace]"))
        .subcommand(command!("create")
            .arg(arg!(-t --target <TARGET>)
                .value_parser(value_parser!(PathBuf))
                .required(true)
                .help("target file generating with compression"))
            .arg(arg!(-s --source <SOURCE>)
                .value_parser(value_parser!(PathBuf))
                .required(true)
                .help("original source text file")))
        .subcommand(command!("search")
            .arg(arg!(-q --query <QUERY>)
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .required(true)
                .help("query string"))
            .arg(arg!(-f --file <FILE>)
                .help("indexed compressed file")
                .value_parser(value_parser!(PathBuf))
                .required(true)))
        .subcommand(command!("expand")
            .arg(arg!(-t --target <TARGET>)
                .value_parser(value_parser!(PathBuf))
                .required(true)
                .help("target file generating with compression"))
            .arg(arg!(-s --source <SOURCE>)
                .value_parser(value_parser!(PathBuf))
                .required(true)
                .help("original source text file")))
    .get_matches();

    let chunk_size = match matches.get_one::<String>("chunk").unwrap().as_str() {
        "4M" => 4*1024*1024, // default 
        "8M" => 8*1024*1024,
        "16M" => 16*1024*1024,
        &_ => todo!(),
    };

    // Get the logging level from the command line, or default to 'info'
    let log_level = match matches.get_one::<String>("log").unwrap().as_str() {
        "error" => Level::Error,
        "warn" => Level::Warn,
        "debug" => Level::Debug,
        "trace" => Level::Trace,
        _ => Level::Info,
    };
    
    // logging
    SimpleLogger::new().with_level(log_level.to_level_filter()).init().unwrap();
    log::debug!("finish argument parsing");

    /*
        This is an expermental code. Do not care of chunks divide lines
     */
    let index_path = matches.get_one::<PathBuf>("index").unwrap();

    match matches.subcommand_name() {
        Some("create") => {
            let subcommand = matches.subcommand_matches("create").unwrap();
            let target_path = subcommand.get_one::<PathBuf>("target").unwrap();
            let source_path = subcommand.get_one::<PathBuf>("source").unwrap();
            log::debug!("target_path:{}",target_path.display());
            log::debug!("source_path:{}",source_path.display());

            let mut index = fs::OpenOptions::new().write(true).create(true).truncate(true).open(index_path)?;
            let mut target = fs::OpenOptions::new().write(true).create(true).truncate(true).open(target_path)?;
            let mut source = fs::File::open(source_path)?;
            create_files(&mut source,&mut target,&mut index, chunk_size)?;
        },
        Some("search") => {
            let subcommand = matches.subcommand_matches("search").unwrap();
            
            let file_path = subcommand.get_one::<PathBuf>("file").unwrap();
            let query =  subcommand.get_one::<String>("query").unwrap();
            let mut index = fs::File::open(index_path)?;
            let mut file = fs::File::open(file_path)?;
            query::query(file.as_raw_fd(), &mut index, &query.to_string(), chunk_size)?;
        },
        Some("expand") => {
            let subcommand = matches.subcommand_matches("expand").unwrap();
            let target_path = subcommand.get_one::<PathBuf>("target").unwrap();
            let source_path = subcommand.get_one::<PathBuf>("source").unwrap();
            log::debug!("target_path:{}",target_path.display());
            log::debug!("source_path:{}",source_path.display());

            let mut index = fs::File::open(index_path)?;
            let mut target = fs::OpenOptions::new().write(true).create(true).truncate(true).open(target_path)?;
            let mut source = fs::File::open(source_path)?;
            expand_file(&mut source,&mut target,&mut index, chunk_size)?;
        },
        Some(_) => {},
        None => {}
    }




    /* 
    let query_string = matches.get_one::<String>("query");
    let file_path = matches.get_one::<PathBuf>("file").unwrap();
    let index_path = matches.get_one::<PathBuf>("index").unwrap();
    
    log::debug!("query = {:?}",query_string);
    match query_string {
        Some(q) =>  
        {
            let mut index = fs::File::open(index_path)?;
            let file_fd = fcntl::open(file_path, nix::fcntl::OFlag::O_RDONLY , stat::Mode::empty())?;
            query(file_fd,&mut index, q.clone(),chunk_size)?;
        },
        None => {
            let mut index = fs::OpenOptions::new().write(true).create(true).open(index_path)?;
            let mut file = fs::File::open(file_path)?;
            create_index(&mut file,&mut index,chunk_size)?;
        } 
    };
    */

    Ok(())
}

