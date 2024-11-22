pub mod create_tx;
pub mod init;

// helper method for formatting pretty please output with newlines:
/*
not good enough rn, maybe later
pub fn format_rust_code(code: String) -> String {
    let mut output = code;

    // List of patterns that should have extra spacing before them
    let patterns = [
        "\nimpl ",
        "\npub struct ",
        "\npub enum ",
        "\n#[derive",
        "\nuse ",
        "\ntype ",
        "\nconst ",
        "\nstatic ",
        "\ntrait ",
        "\nfn ",
        "\nmod ",
        "\n/// ",
    ];

    // Add extra newline before each pattern unless it's at the start of the file
    for pattern in patterns {
        // Don't add extra newline if it's the first non-empty line
        let parts: Vec<&str> = output.splitn(2, pattern).collect();
        if parts.len() == 2 && !parts[0].trim().is_empty() {
            output = output.replace(pattern, &format!("\n{}", pattern));
        }
    }

    // Remove any instances of 3 or more consecutive newlines, replacing with 2
    while output.contains("\n\n\n") {
        output = output.replace("\n\n\n", "\n\n");
    }

    output
}
 */
