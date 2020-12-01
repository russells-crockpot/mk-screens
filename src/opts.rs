use std::{
    env::{set_var, var_os as get_var},
    num::{ParseFloatError, ParseIntError},
    path::PathBuf,
    str::FromStr as _,
};
use structopt::StructOpt;

fn parse_skip(src: &str) -> Result<f64, ParseFloatError> {
    Ok(f64::from_str(src)? / 100.0)
}

fn parse_jobs(src: &str) -> Result<usize, ParseIntError> {
    if src.is_empty() {
        let cpus = num_cpus::get();
        if cpus == 1 {
            Ok(cpus)
        } else {
            Ok(cpus / 2)
        }
    } else {
        Ok(usize::from_str(src)?)
    }
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    name=env!("CARGO_PKG_NAME"),
    version=env!("CARGO_PKG_VERSION"),
    rename_all="kebab",
    author=env!("CARGO_PKG_AUTHORS"),
    about=env!("CARGO_PKG_DESCRIPTION")
)]
pub struct Opts {
    #[structopt(short = "k", long)]
    pub keep_files: bool,
    #[structopt(short = "f", long)]
    pub force: bool,
    #[structopt(short = "j", long,
        default_value = "",
        parse(try_from_str = parse_jobs),
    )]
    pub jobs: usize,
    #[structopt(short = "w", long, default_value = "3840")]
    pub width: u32,
    #[structopt(short, long)]
    pub verbose: bool,
    //#[structopt(short, long, default_value = "25")]
    #[structopt(short, long, default_value = "3")]
    pub columns: u32,
    //#[structopt(short, long, default_value = "8")]
    #[structopt(short, long, default_value = "3")]
    pub rows: u32,
    #[structopt(short, long,
        default_value="5",
        help="The percent of amount of the video to skip at the beginning of the file.",
        parse(try_from_str = parse_skip),
    )]
    //TODO Validate value.
    pub skip: f64,
    #[structopt(short, long, default_value = "screens")]
    pub out_dir: PathBuf,
    #[structopt(name = "FILE_OR_DIR", default_value = ".", multiple = true)]
    pub input: Vec<PathBuf>,
}

impl Opts {
    pub fn num_captures(&self) -> u32 {
        self.columns * self.rows
    }
}

impl Default for Opts {
    fn default() -> Opts {
        let opts = Opts::from_args();
        if opts.verbose {
            set_var("RUST_LOG", "mk_screens=debug");
        } else if get_var("RUST_LOG") == None {
            set_var("RUST_LOG", "mk_screens=info");
        }
        opts
    }
}
