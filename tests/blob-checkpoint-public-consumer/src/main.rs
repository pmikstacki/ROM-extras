mod fixtures;
mod native;
mod tracked;
use rom_blob_recovery::Context;
use std::{path::PathBuf, time::Duration};
fn context() -> Context {
    Context::new(
        tokio::time::Instant::now() + Duration::from_secs(30),
        tokio_util::sync::CancellationToken::new(),
    )
}
fn main() {
    let root = PathBuf::from(
        std::env::var("ROM_EXTRAS_BLOB_CHECKPOINT_RUN")
            .expect("explicit private run directory required"),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let args: Vec<_> = std::env::args().skip(1).collect();
    runtime.block_on(async{
        if args.is_empty(){native::run(&root).await;println!("blob checkpoint public consumer passed; retained native files and private inventories");}
        else{assert_eq!(args.len(),2);native::child(&root,&args[0],&args[1]).await;}
    });
}
