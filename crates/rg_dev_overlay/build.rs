use std::error::Error;

use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    EmitBuilder::builder()
        .build_date()
        .git_describe(false, false, None)
        .git_commit_date()
        .emit()?;
    Ok(())
}
