//! Same process-restart probe on Windows, Linux and Android; no network listener.
#![forbid(unsafe_code)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("Usage: check-automerge EMPTY_PROBE_DIRECTORY prepare|resume|conflicts-prepare|conflicts-resolve|conflicts-resume|containers-prepare|containers-resolve|containers-resume|order-prepare|order-resolve|order-resume|tags-prepare|tags-resolve|tags-resume|identity-prepare|identity-rename|identity-resume".into());
    }
    zedone_prototype_core::automerge_probe::run(
        std::path::Path::new(&args[0]),
        args[1].to_str().ok_or("Phase must be UTF-8")?,
    )
}
