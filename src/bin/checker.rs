use pace26checker::checks::checker::{
    CheckerError, check_instance_and_solution, check_instance_only,
};
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;
use tracing::error;

#[derive(StructOpt)]
pub struct Arguments {
    #[structopt()]
    pub instance: PathBuf,

    #[structopt()]
    pub solution: Option<PathBuf>,

    #[structopt(short, long)]
    pub quiet: bool,

    #[structopt(short, long)]
    pub paranoid: bool,
}

fn check(args: &Arguments) -> Result<(), CheckerError> {
    if let Some(solution_path) = args.solution.as_ref() {
        let (_inst, solution, _forest) =
            check_instance_and_solution(&args.instance, solution_path, args.paranoid, false)?;
        println!("Trees in solution: {}", solution.num_trees());
    } else {
        let _ = check_instance_only(&args.instance, args.paranoid)?;
    }
    Ok(())
}

fn main() {
    let args = Arguments::from_args();

    if !args.quiet {
        tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_max_level(tracing::Level::INFO)
            .without_time()
            .init();
    }

    if let Err(e) = check(&args) {
        error!("{e}");
        exit(1)
    }
}
