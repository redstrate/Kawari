use std::{cmp::Ordering, fs::read_dir};

pub fn list_patch_files(dir_path: &str) -> Vec<String> {
    // If the dir doesn't exist, pretend there is no patch files
    let Ok(dir) = read_dir(dir_path) else {
        return Vec::new();
    };
    let mut entries: Vec<_> = dir.flatten().collect();
    entries.sort_by_key(|dir| dir.path());
    let mut game_patches: Vec<_> = entries
        .into_iter()
        .flat_map(|entry| {
            let Ok(meta) = entry.metadata() else {
                return vec![];
            };
            if meta.is_dir() {
                return vec![];
            }
            if meta.is_file() && entry.file_name().to_str().unwrap().contains(".patch") {
                return vec![entry.path()];
            }
            vec![]
        })
        .collect();
    game_patches.sort_by(|a, b| {
        // Ignore H/D in front of filenames
        let a_path = a
            .as_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        if a_path.starts_with("H") {
            return Ordering::Less;
        }
        let b_path = b
            .as_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        /*if b_path.starts_with("H") {
            return Ordering::Greater;
        }*/
        a_path.partial_cmp(&b_path).unwrap()
    }); // ensure we're actually installing them in the correct order
    game_patches
        .iter()
        .map(|x| x.file_stem().unwrap().to_str().unwrap().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[macro_export]
    macro_rules! patch_tests_dir {
        ($rel_path:literal) => {
            concat!("../../resources/data/tests/patch/", $rel_path)
        };
    }

    #[test]
    fn test_list_patches() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(patch_tests_dir!("files"));

        let patch_files = list_patch_files(d.as_path().to_str().unwrap());
        assert_eq!(
            patch_files,
            vec!["H2001.00.00.0000", "D2000.00.00.0000", "D2000.00.00.0001"]
        );
    }
}
