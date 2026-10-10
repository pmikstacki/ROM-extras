use rom_extras_maintenance::{Backend, Error, Limits, inspect, restore};
use std::path::Path;
fn execute(args: &[String]) -> Result<String, Error> {
    if args.len() < 3 || args.len() > 8 || args.iter().any(|a| a.len() > 4096) {
        return Err(Error::Invalid);
    }
    let backend = match args[1].as_str() {
        "sqlite" => Backend::Sqlite,
        "redb" => Backend::Redb,
        _ => return Err(Error::Unsupported),
    };
    let base = match args[0].as_str() {
        "inspect" => 3,
        "restore" => 4,
        _ => return Err(Error::Unsupported),
    };
    if args.len() < base || !(args.len() - base).is_multiple_of(2) {
        return Err(Error::Invalid);
    }
    let (mut bytes, mut records) = (128 * 1024 * 1024, 400000);
    let (mut seen_bytes, mut seen_records) = (false, false);
    for pair in args[base..].as_chunks::<2>().0 {
        let value = pair[1].parse::<usize>().map_err(|_| Error::Invalid)?;
        match pair[0].as_str() {
            "--max-bytes" if !seen_bytes => {
                bytes = value;
                seen_bytes = true;
            }
            "--max-records" if !seen_records => {
                records = value;
                seen_records = true;
            }
            _ => return Err(Error::Invalid),
        }
    }
    let limits = Limits::new(bytes, records)?;
    if base == 3 {
        serde_json::to_string(&inspect(backend, Path::new(&args[2]), limits)?)
            .map_err(|_| Error::Unavailable)
    } else {
        let storage = restore(backend, Path::new(&args[2]), Path::new(&args[3]), limits)?;
        drop(storage);
        Ok("{\"restored\":true,\"external_blobs_included\":false,\"external_deliveries_included\":false,\"host_cutover_required\":true}".into())
    }
}
pub(crate) fn run() -> i32 {
    let args: Result<Vec<_>, _> = std::env::args_os()
        .skip(1)
        .take(9)
        .map(|a| a.into_string().map_err(|_| Error::Invalid))
        .collect();
    match args.and_then(|args| execute(&args)) {
        Ok(report) => {
            println!("{report}");
            0
        }
        Err(error) => {
            eprintln!("maintenance:{error}");
            if error == Error::Unknown { 3 } else { 2 }
        }
    }
}
