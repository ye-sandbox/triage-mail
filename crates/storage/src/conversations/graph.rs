use std::collections::{BTreeMap, HashMap};

/// INVARIANT: Header IDs are case-sensitive, untrusted opaque values, never identities.
pub(super) fn header_id(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let id = raw
        .strip_prefix('<')
        .and_then(|v| v.strip_suffix('>'))
        .unwrap_or(raw);
    let (local, domain) = id.split_once('@')?;
    (id.len() <= 998
        && !local.is_empty()
        && !domain.is_empty()
        && id
            .bytes()
            .all(|c| c.is_ascii_graphic() && !b"<>(),;:\\\"[]".contains(&c))
        && !domain.contains('@'))
    .then(|| id.to_string())
}
pub(super) fn references(raw: &str) -> Vec<String> {
    if raw.len() > 65536 {
        return Vec::new();
    }
    let parts: Vec<_> = raw.split_whitespace().collect();
    if parts.len() > 64 {
        return Vec::new();
    }
    parts
        .into_iter()
        .map(header_id)
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}
#[derive(Clone)]
pub(super) struct Input {
    pub id: String,
    pub header: Option<String>,
    pub parent: Option<String>,
    pub refs: Vec<String>,
}
fn root(parents: &mut [usize], mut node: usize) -> usize {
    while parents[node] != node {
        parents[node] = parents[parents[node]];
        node = parents[node];
    }
    node
}
/// Linear-space union/find tolerates arbitrary cycles without recursion or dropped records.
pub(super) fn group(inputs: &[Input]) -> Vec<(String, String)> {
    let mut headers: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, input) in inputs.iter().enumerate() {
        if let Some(id) = &input.header {
            headers.entry(id).or_default().push(i);
        }
    }
    let mut parents: Vec<_> = (0..inputs.len()).collect();
    let mut missing = HashMap::new();
    for (i, input) in inputs.iter().enumerate() {
        let usable = |id: &&String| {
            input.header.as_ref() != Some(*id)
                && headers.get(id.as_str()).is_none_or(|rows| rows.len() == 1)
        };
        let candidate = input
            .parent
            .as_ref()
            .filter(usable)
            .or_else(|| input.refs.iter().rev().find(usable));
        let Some(id) = candidate else {
            continue;
        };
        let target = headers.get(id.as_str()).map(|v| v[0]).unwrap_or_else(|| {
            *missing.entry(id.as_str()).or_insert_with(|| {
                let node = parents.len();
                parents.push(node);
                node
            })
        });
        let a = root(&mut parents, i);
        let b = root(&mut parents, target);
        parents[a] = b;
    }
    // INVARIANT: The smallest canonical record, not an attacker-controlled header, names a group.
    let mut names = BTreeMap::<usize, &str>::new();
    for (i, input) in inputs.iter().enumerate() {
        let r = root(&mut parents, i);
        names
            .entry(r)
            .and_modify(|v| *v = (*v).min(&input.id))
            .or_insert(&input.id);
    }
    inputs
        .iter()
        .enumerate()
        .map(|(i, input)| (input.id.clone(), names[&root(&mut parents, i)].to_string()))
        .collect()
}
