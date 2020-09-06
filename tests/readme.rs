/// Test that tests/example.rs is faithfully reproduced in the readme.
#[test]
fn test_readme_example() {
    let source = include_str!("../tests/example.rs");

    // Extract the example code between two comments and trim
    // leading whitespace
    let mut example = Vec::new();
    let mut copy = false;
    for line in source.lines() {
        if line.contains("Begin readme example") {
            copy = true;
        } else if line.contains("End readme example") {
            break;
        } else if copy {
            example.push(line.trim_start());
        }
    }
    let example = example.join("\n");

    let readme = include_str!("../README.md");
    assert!(readme.contains(&example));
}
