mod cli;
mod db;
mod editor;
mod models;
mod ui;

fn main() -> anyhow::Result<()> {
    let conn = db::init()?;
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        cli::run(&conn)
    } else {
        ui::run(&conn)
    }
}
