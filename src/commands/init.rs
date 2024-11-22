use crate::templates;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn create_project(project_name: &str) -> Result<()> {
    Command::new("cargo")
        .args(["new", project_name])
        .output()
        .context("Failed to create new cargo project")?;

    let project_dir = Path::new(project_name);
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir).context("Failed to create src directory")?;

    write_template_files(&src_dir)?;

    let cargo_content = templates::CARGO_TEMPLATE.replace("shard-template", project_name);
    fs::write(project_dir.join("Cargo.toml"), cargo_content)
        .context("Failed to update Cargo.toml")?;
    fs::write(
        project_dir.join("Cargo.lock"),
        templates::CARGO_LOCK_TEMPLATE,
    )
    .context("Failed to create Cargo.lock")?;

    println!("✨ Created new rollup project: {}", project_name);
    Ok(())
}

fn write_template_files(src_dir: &Path) -> Result<()> {
    let files = [
        ("lib.rs", templates::LIB_RS),
        ("main.rs", templates::MAIN_RS),
        ("node.rs", templates::NODE_RS),
        ("state.rs", templates::STATE_RS),
        ("tx.rs", templates::TX_RS),
        ("webserver.rs", templates::SERVER_RS),
    ];

    for (filename, content) in files {
        fs::write(src_dir.join(filename), content)
            .with_context(|| format!("Failed to create {}", filename))?;
    }

    Ok(())
}
