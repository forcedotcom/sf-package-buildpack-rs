use regex::Regex;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;

pub fn find_one_file(path: &Path, word: &str) -> bool {
    for (_i, e) in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .enumerate()
    {
        if e.metadata().unwrap().is_file() {
            if let Some(b) = self::contains(e.path(), &annotation_pattern(word)) {
                if b {
                    return true;
                }
            }
        }
    }
    false
}

pub fn read_file_to_string<P: AsRef<Path>>(src: P) -> Option<String> {
    return match File::open(src) {
        Ok(mut file) => {
            let mut content = String::new();
            match file.read_to_string(&mut content) {
                Ok(_usize) => Option::Some(content),
                Err(_e) => Option::None,
            }
        }
        Err(_) => Option::None,
    };
}

fn annotation_pattern(s: &str) -> Regex {
    Regex::new(format!(r"@(\b)(?i:{})(\b)", s).as_str()).unwrap()
}

fn contains<P: AsRef<Path>>(src: P, pat: &Regex) -> Option<bool> {
    return match self::read_file_to_string(src) {
        Some(s) => Option::Some(pat.is_match(&s)),
        None => Option::None,
    };
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;
    use std::io::Write;

    pub fn write_file(ignored_content: &str, f: &PathBuf) -> anyhow::Result<(), anyhow::Error> {
        match File::create(f) {
            Err(why) => Err(anyhow::Error::new(why)),
            Ok(mut file) => {
                file.write_all(ignored_content.as_bytes()).unwrap();
                Ok(())
            }
        }
    }

    #[test]
    fn test_regex_pattern() {
        let pat = annotation_pattern("IsTest");
        assert!(pat.is_match("@IsTest\npublic class Yoda {}"));
        assert!(pat.is_match("@isTest\npublic class Yoda {}"));
        assert!(pat.is_match(
            "@ISTEST public class Yoda {\n  @istest\nprivate static void testIt() {}\n}"
        ));
        assert!(!pat.is_match("@IsTesty\npublic class Yoda {}"));
        assert!(!pat.is_match("IsTest public class Yoda {}"));
        assert!(!pat.is_match("@ IsTest public class Yoda {}"));
    }

    #[test]
    fn test_find_one_file() {
        let found_content = r"
@IsTest
public class TestTests {
    @IsTest
    static void testBehavior() {
        new Test().gimmeString();
    }
}
        ";

        let ignored_content = r"
public with sharing class Test {
    public String gimmeString() {
        return 'Hi there world';
    }
}
        ";
        let temp_dir = tempdir().unwrap();
        let test_dir = temp_dir.path();
        for f in ["file1.cls", "file2.cls", "file3.cls"] {
            write_file(ignored_content, &test_dir.join(f)).unwrap();
        }
        for f in ["file4.cls", "file5.cls"] {
            write_file(found_content, &test_dir.join(f)).unwrap();
        }

        assert!(
            find_one_file(test_dir, "IsTest"),
            "Should have found at least one test file, we wrote several"
        );
    }

    #[test]
    fn test_find_test_files() {
        let app_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sf-package/force-app")
            .canonicalize()
            .unwrap();
        println!("Reading {:?}", app_dir);
        assert!(
            find_one_file(app_dir.as_path(), "IsTest"),
            "Should have found at least one test file in {:?}",
            app_dir
        );
    }
}
