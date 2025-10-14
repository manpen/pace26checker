use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Opts {
    #[structopt()]
    pub instance: PathBuf,

    #[structopt()]
    pub solution: Option<PathBuf>,

    #[structopt(short, long)]
    pub quiet: bool,

    #[structopt(short, long)]
    pub paranoid: bool,
}

impl Opts {
    pub fn process() -> Self {
        let opts = Opts::from_args();

        env_logger::builder()
            .filter_level(if opts.quiet {
                log::LevelFilter::Error
            } else {
                log::LevelFilter::Info
            })
            .init();

        opts
    }
}
