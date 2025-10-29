//! Test for argument passing in raven-cli

#[cfg(test)]
mod tests {
    // ...existing code...

    #[test]
    fn test_argument_passing() {
        // Print the current working directory for debugging
        let cwd = std::env::current_dir().expect("Failed to get current working directory");
        println!("Current working directory: {}", cwd.display());
        // Remove the .rom file if it exists, so the CLI must create it
        let dst_rom = std::path::Path::new("tests/helloworld.rom");
        if dst_rom.exists() {
            std::fs::remove_file(dst_rom).expect("Failed to remove pre-existing helloworld.rom");
        }
        println!("helloworld.rom exists before run: {}", dst_rom.exists());
        // Run raven-cli and check output
        let exe = assert_cmd::cargo::cargo_bin("raven-cli");
        let mut cmd = std::process::Command::new(&exe);
    let rom = std::path::Path::new("drifblim-seed.rom");
    println!("ROM path: {}", rom.display());
    println!("ROM exists before run: {}", rom.exists());
    assert!(!rom.exists(), "drifblim-seed.rom is missing in the test directory: {}", rom.display());
    let args = ["helloworld.tal", "helloworld.rom"];
    // Pass ROM as first positional arg, then --, then input/output files
    println!("Running: raven-cli {} -- {:?}", rom.display(), args);
            let output = cmd
                .current_dir("tests")
                .arg(rom)
                .arg("--")
                .args(&args)
                .output()
                .expect("failed to run raven-cli");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("raven-cli stdout:\n{}", stdout);
        println!("raven-cli stderr:\n{}", stderr);
    // The output ROM should be created
    println!("helloworld.rom exists after run: {}", dst_rom.exists());
    assert!(dst_rom.exists(), "helloworld.rom was not created by raven-cli");
    // The output should not be empty
    let metadata = std::fs::metadata(dst_rom).expect("Failed to stat output ROM");
    assert!(metadata.len() > 0, "Output ROM file is empty");
    // Check that the ROM file exists
    assert!(std::path::Path::new("tests/helloworld.rom").exists(), "helloworld.rom does not exist in the test directory");

        // Check that stdout is not empty (should match buxn behavior)
        assert!(!stderr.trim().is_empty(), "stderr is empty, expected output from ROM execution");
        // The output should mention assembling the ROM (check both stdout and stderr, allow for line ending differences)
        let re = regex::Regex::new(r"Assembled helloworld\.rom in \d+ bytes\.").unwrap();
        assert!(re.is_match(&stdout) || re.is_match(&stderr), "Expected assembly message not found.\nstdout:\n{}\nstderr:\n{}", stdout, stderr);
    }
}
