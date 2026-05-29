use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use walkdir::WalkDir;

//choose driver struct(Cargo.toml must add like 'rbdc-*** = { version = "4.5" }')
//database_struct: "rbdc_sqlite::Driver{}",
//database_struct: "rbdc_mysql::Driver{}",
//database_struct: "rbdc_mssql::Driver{}",
//database_struct: "rbdc_pg::Driver{}",
//database_struct: "rbdc_sqlite::Driver{}",
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApplicationConfig {
    pub db_url: String,
}

fn main() {
    ensure_legacy_driver_path_detector_regression_guard();

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set by Cargo");
    let out_dir = Path::new(&out_dir);

    let js_data = include_str!("config/application.json5");
    let config: ApplicationConfig = json5::from_str(js_data).expect("load config file fail");
    let mut data = String::new();
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out_dir.join("driver.rs"))
        .unwrap();
    _ = f.read_to_string(&mut data);

    let db_index = config
        .db_url
        .find(":")
        .expect("db_url must be '<database>://xxxx'");
    let mut db_name = &config.db_url[..db_index];
    if db_name == "postgres" {
        db_name = "pg";
    }
    let driver_path = format!("rbdc_{}::Driver{}", db_name, "{}");
    println!("driver_path={}", driver_path);
    _ = f.set_len(0);
    f.write_all(driver_path.as_bytes()).unwrap();
    f.flush().unwrap();

    //unwrap check
    unwrap_check("src/controller");
    unwrap_check("src/domain/dto");
    unwrap_check("src/domain/vo");
    unwrap_check("src/middleware");
    unwrap_check("src/service");
    unwrap_check("src/util");
    for dir in ["src", "tests", "examples", "benches"] {
        forbid_legacy_driver_path(dir);
    }
}

const LEGACY_DRIVER_CANONICAL: &str = "target/driver.rs";

fn is_driver_path_boundary_char(ch: char) -> bool {
    matches!(
        ch,
        '"' | '\'' | '(' | ')' | '[' | ']' | ',' | ';' | ' ' | '\t' | '/'
    )
}

fn has_driver_rs_boundary(normalized: &str, start: usize) -> bool {
    let end = start + LEGACY_DRIVER_CANONICAL.len();

    let has_prefix_boundary = if start == 0 {
        true
    } else {
        normalized[..start]
            .chars()
            .next_back()
            .is_some_and(is_driver_path_boundary_char)
    };

    if !has_prefix_boundary {
        return false;
    }

    if end >= normalized.len() {
        return true;
    }
    normalized[end..]
        .chars()
        .next()
        .is_some_and(is_driver_path_boundary_char)
}

fn find_legacy_driver_path(line: &str) -> Option<usize> {
    let mut normalized = line.replace("\\\"", "\"");
    normalized = normalized.replace("\\\\", "/").replace('\\', "/");
    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }

    let mut search_from = 0;
    while let Some(relative_start) = normalized[search_from..].find(LEGACY_DRIVER_CANONICAL) {
        let start = search_from + relative_start;
        if has_driver_rs_boundary(&normalized, start) {
            let target_ordinal = normalized[..start].match_indices("target").count();
            if let Some((idx, _)) = line.match_indices("target").nth(target_ordinal) {
                return Some(idx);
            }
        }
        search_from = start + 1;
    }

    None
}

fn ensure_legacy_driver_path_detector_regression_guard() {
    let positive_cases = [
        r#"include!(\"target/driver.rs\")"#,
        r#"include!(\"target\driver.rs\")"#,
        r#"include!(\"target\\driver.rs\")"#,
        r#"include!(\"./target//driver.rs\")"#,
        r#"include!(\"target\\//driver.rs\")"#,
        r#"const SHADOW: &str = \"mytarget/driver.rs\"; include!(\"target/driver.rs\")"#,
    ];

    for line in positive_cases {
        assert!(
            find_legacy_driver_path(line).is_some(),
            "legacy driver path detector missed positive case: {line}"
        );
    }

    let shadow_case =
        r#"const SHADOW: &str = \"mytarget/driver.rs\"; include!(\"target/driver.rs\")"#;
    let expected_shadow_index = shadow_case
        .match_indices("target")
        .nth(1)
        .map(|(idx, _)| idx)
        .expect("expected second target in shadow_case");
    assert_eq!(
        find_legacy_driver_path(shadow_case),
        Some(expected_shadow_index),
        "legacy driver path detector should report canonical match index, not shadow prefix"
    );

    let negative_cases = [
        r#"include!(concat!(env!(\"OUT_DIR\"), \"/driver.rs\"))"#,
        r#"const DRIVER: &str = \"driver.rs\";"#,
        r#"const DRIVER: &str = \"target/driver.ts\";"#,
        r#"const DRIVER: &str = \"target/driver.rs.bak\";"#,
        r#"include!(\"mytarget/driver.rs\")"#,
        r#"include!(\"target/driver.rs.tmp\")"#,
    ];

    for line in negative_cases {
        assert!(
            find_legacy_driver_path(line).is_none(),
            "legacy driver path detector false positive: {line}"
        );
    }
}

//format print
fn emit_rust_error(path: &str, line_no: usize, col: usize, line: &str, msg: &str) {
    println!("error: {}", msg);
    println!("  --> {}:{}:{}", path, line_no, col);
    println!("   |");
    println!("{:>3} | {}", line_no, line);
    // underline line
    print!("   | ");
    // print (col-1) spaces
    for _ in 0..col {
        print!(" ");
    }
    // caret underline
    println!("^---- {}", msg);
}

//check server code have .unwrap()
fn unwrap_check(dir: &str) {
    for entry in WalkDir::new(dir) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        let path_str = path.to_string_lossy().to_string();

        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let reader = BufReader::new(file);
        let mut in_test_block = false;
        let mut brace_depth = 0;
        let mut test_block_start_line = 0;
        let mut previous_line_ends_with_lock = false;

        for (line_no, line) in reader.lines().enumerate() {
            let line_no = line_no + 1;
            let line = line.unwrap_or_default();

            if !in_test_block && (line.contains("#[cfg(test)]") || line.contains("#[test]")) {
                in_test_block = true;
                test_block_start_line = line_no;
                brace_depth = 0;
            }

            if in_test_block {
                for c in line.chars() {
                    if c == '{' {
                        brace_depth += 1;
                    } else if c == '}' {
                        if brace_depth > 0 {
                            brace_depth -= 1;
                        }
                    }
                }

                if brace_depth == 0 && line_no > test_block_start_line {
                    in_test_block = false;
                }

                continue;
            }

            if let Some(col) = line.find(".unwrap()") {
                emit_rust_error(&path_str, line_no, col + 1, &line, "found .unwrap()");
                std::process::exit(1);
            }

            if let Some(col) = line.find("panic!(") {
                emit_rust_error(&path_str, line_no, col + 1, &line, "found panic!()");
                std::process::exit(1);
            }

            if let Some(col) = line.find(".expect(") {
                if !line.contains(".lock().expect(") && !previous_line_ends_with_lock {
                    emit_rust_error(&path_str, line_no, col + 1, &line, "found .expect()");
                    std::process::exit(1);
                }
            }

            previous_line_ends_with_lock = line.trim().ends_with(".lock()");
        }
    }
}

fn forbid_legacy_driver_path(dir: &str) {
    for entry in WalkDir::new(dir) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }

        let path_str = path.to_string_lossy().to_string();
        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let reader = BufReader::new(file);
        for (line_no, line) in reader.lines().enumerate() {
            let line_no = line_no + 1;
            let line = line.unwrap_or_default();

            if let Some(col) = find_legacy_driver_path(&line) {
                emit_rust_error(
                    &path_str,
                    line_no,
                    col + 1,
                    &line,
                    "found legacy fixed path target/driver.rs, use OUT_DIR include instead",
                );
                std::process::exit(1);
            }
        }
    }
}
