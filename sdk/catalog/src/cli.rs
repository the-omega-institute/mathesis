use super::*;

fn parse_arg(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String> {
    args.next()
        .ok_or_else(|| msg(format!("missing value for {name}")))
}
/// Command-line entry point, kept small so library functions are directly testable.
pub fn run_cli<I: Iterator<Item = String>>(mut args: I) -> Result<()> {
    let command = args
        .next()
        .ok_or_else(|| msg("command required: snapshot, fetch, render or check"))?;
    let mut api = None;
    let mut output = None;
    let mut snapshot_root = None;
    let mut organization = None;
    let mut display_name = "Mathesis".to_owned();
    let mut site = "https://mathesis-omega-research.apps.chrono-ai.fun".to_owned();
    let mut mapping = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--api-base" => api = Some(parse_arg(&mut args, "--api-base")?),
            "--output" => output = Some(PathBuf::from(parse_arg(&mut args, "--output")?)),
            "--snapshot" => {
                snapshot_root = Some(PathBuf::from(parse_arg(&mut args, "--snapshot")?))
            }
            "--organization" => organization = Some(parse_arg(&mut args, "--organization")?),
            "--display-name" => display_name = parse_arg(&mut args, "--display-name")?,
            "--site-base" => site = parse_arg(&mut args, "--site-base")?,
            "--mapping" => mapping = Some(PathBuf::from(parse_arg(&mut args, "--mapping")?)),
            _ => return Err(msg(format!("unknown argument {arg}"))),
        }
    }
    match command.as_str() {
        "snapshot" => {
            let base = api.ok_or_else(|| msg("snapshot needs --api-base"))?;
            let out = output.ok_or_else(|| msg("snapshot needs --output"))?;
            let m = snapshot(&base, &out)?;
            println!(
                "snapshot: {} works, {} directory nodes, {} listing groups",
                m.collected_works,
                m.directory_nodes,
                m.listings.len()
            );
            Ok(())
        }
        "fetch" => {
            let root = snapshot_root
                .or(output)
                .ok_or_else(|| msg("fetch needs --snapshot"))?;
            fetch_files(&root)
        }
        "render" => {
            let root = snapshot_root.ok_or_else(|| msg("render needs --snapshot"))?;
            let out = output.ok_or_else(|| msg("render needs --output"))?;
            let organization = organization.ok_or_else(|| msg("render needs --organization"))?;
            render(
                &root,
                &out,
                &organization,
                &display_name,
                &site,
                mapping.as_deref(),
            )
        }
        "check" => {
            let out = output.ok_or_else(|| msg("check needs --output"))?;
            check(&out)
        }
        _ => Err(msg(format!("unknown command {command}"))),
    }
}
