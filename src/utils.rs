use colored::Colorize;
use difference::{Changeset, Difference};
use serde_json::Value;

#[derive(Clone)]
enum PathSegment {
    Key(String),
    Index(usize),
}

/// Converts a vector of PathSegments into a JSON Pointer string.
/// (Escapes '~' as "~0" and '/' as "~1" for full JSON Pointer compliance.)
fn path_to_pointer(path: &[PathSegment]) -> String {
    let mut pointer = String::new();
    for seg in path {
        match seg {
            PathSegment::Key(s) => {
                let escaped = s.replace('~', "~0").replace('/', "~1");
                pointer.push('/');
                pointer.push_str(&escaped);
            }
            PathSegment::Index(i) => {
                pointer.push('/');
                pointer.push_str(&i.to_string());
            }
        }
    }
    pointer
}

/// Iteratively traverses `expected` to find all occurrences of the string "ignore".
/// For each such occurrence:
///   - The corresponding field in `resp` is set to the string "ignore".
///   - If the "ignore" is found as the value of a key in an object (i.e. not at the root),
///     that key is removed from the expected object.
/// If the entire expected value is `"ignore"`, then the whole `resp` is set to `"ignore"`.
pub(crate) fn sanitize_resp_by_expected(resp: &mut Value, expected: &mut Value) {
    // Use a stack to store paths (from the root) into the JSON trees.
    let mut stack: Vec<Vec<PathSegment>> = vec![vec![]];

    while let Some(path) = stack.pop() {
        let pointer = path_to_pointer(&path);
        if let Some(exp_node) = expected.pointer_mut(&pointer) {
            // If the expected node is a string "ignore", update the resp accordingly.
            if let Value::String(s) = exp_node {
                if s == "ignore" {
                    if let Some(resp_node) = resp.pointer_mut(&pointer) {
                        *resp_node = Value::String("ignore".to_string());
                    }

                    continue; // Stop processing this branch.
                }
            }
            // If the expected node is an object, iterate through its keys.
            if exp_node.is_object() {
                if let Value::Object(map) = exp_node {
                    // Collect keys first as we might modify the object while iterating.
                    let keys: Vec<String> = map.keys().cloned().collect();
                    for key in keys {
                        let mut new_path = path.clone();
                        new_path.push(PathSegment::Key(key));
                        stack.push(new_path);
                    }
                }
            }
            // If the expected node is an array, iterate through its indices.
            else if exp_node.is_array() {
                if let Value::Array(arr) = exp_node {
                    for i in 0..arr.len() {
                        let mut new_path = path.clone();
                        new_path.push(PathSegment::Index(i));
                        stack.push(new_path);
                    }
                }
            }
        }
    }
}

pub(crate) fn remove_nulls(value: &mut Value) {
    match value {
        Value::Object(map) => {
            // Collect keys that have null values.
            let keys_to_remove: Vec<String> = map
                .iter()
                .filter_map(|(k, v)| if v.is_null() { Some(k.clone()) } else { None })
                .collect();
            // Remove those keys.
            for key in keys_to_remove {
                map.remove(&key);
            }
            // Recurse into remaining values.
            for v in map.values_mut() {
                remove_nulls(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                remove_nulls(v);
            }
        }
        _ => {}
    }
}

pub(crate) fn remove_one_leading_whitespace(s: &str) -> String {
    let mut chars = s.chars();
    if let Some(first) = chars.next() {
        if first.is_whitespace() {
            return chars.as_str().to_string();
        }
    }
    s.to_string()
}

pub(crate) fn pretty_print_diff(expected: &str, received: &str) -> String {
    let mut ret = String::new();
    let changeset = Changeset::new(expected, received, "\n");

    for i in 0..changeset.diffs.len() {
        match changeset.diffs[i] {
            Difference::Same(ref x) => {
                ret = format!("{}{}", ret, x.white());
            }
            Difference::Add(ref x) => {
                ret = format!(
                    "{}\n{}{}\n",
                    ret,
                    "+".green(),
                    remove_one_leading_whitespace(x).green()
                );
            }
            Difference::Rem(ref x) => {
                ret = format!(
                    "{}\n{}{}",
                    ret,
                    "-".red(),
                    remove_one_leading_whitespace(x).red()
                );
            }
        }
    }

    ret
}
