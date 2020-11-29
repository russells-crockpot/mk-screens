use std::{
    path::PathBuf,
    num::ParseFloatError,
    str::FromStr as _,
    env::{
        set_var,
        var_os as get_var,
    },
};
use structopt::StructOpt;

fn parse_skip(src: &str) -> Result<f64, ParseFloatError> {
    Ok(f64::from_str(src)? / 100.0)
}

#[derive(Debug, StructOpt)]
#[structopt(
    name=env!("CARGO_PKG_NAME"),
    version=env!("CARGO_PKG_VERSION"),
    rename_all="kebab",
    author=env!("CARGO_PKG_AUTHORS"),
    about=env!("CARGO_PKG_DESCRIPTION")
)]
pub struct Opts {
    #[structopt(short="k", long)]
    pub keep_files: bool,
    #[structopt(short="w", long)]
    pub overwrite: bool,
    #[structopt(short, long)]
    pub verbose: bool,
    #[structopt(short, long, default_value="14")]
    pub columns: u16,
    #[structopt(short, long, default_value="14")]
    pub rows: u16,
    #[structopt(short, long,
        default_value="5",
        help="The percent of amount of the video to skip at the beginning of the file.",
        parse(try_from_str = parse_skip),
    )]
    //TODO Validate value.
    pub skip: f64,
    #[structopt(short, long, default_value="screens")]
    pub out_dir: PathBuf,
    #[structopt(name="FILE_OR_DIR", default_value=".")]
    pub input: PathBuf,
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
