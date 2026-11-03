//! Lexicographic paths with object-ID tie-breaks, without floating-point rounding.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(super) struct Rank(Vec<(u32, String)>);

impl Rank {
    pub(super) fn initial(digit: u32, id: &str) -> Self {
        Self(vec![(digit, id.to_owned())])
    }
}

pub(super) fn between(left: Option<&Rank>, right: Option<&Rank>, id: &str) -> Result<Rank> {
    if left.zip(right).is_some_and(|(l, r)| l >= r) {
        return Err("Invalid rank interval".into());
    }
    let l = left.map(|v| v.0.as_slice()).unwrap_or_default();
    let r = right.map(|v| v.0.as_slice()).unwrap_or_default();
    let mut result = Vec::new();
    let mut i = 0;
    loop {
        match (l.get(i), r.get(i)) {
            (Some(a), Some(b)) if a == b => result.push(a.clone()),
            (None, Some(b)) if *b == (0, String::new()) => result.push(b.clone()),
            (a, Some(b)) if b.0.saturating_sub(a.map_or(0, |a| a.0)) > 1 => {
                let low = a.map_or(0, |a| a.0);
                result.push((low + (b.0 - low) / 2, id.to_owned()));
                break;
            }
            (None, Some(_)) => {
                result.push((0, String::new()));
                result.push((u32::MAX / 2, id.to_owned()));
                break;
            }
            _ => {
                result.extend_from_slice(&l[i.min(l.len())..]);
                result.push((u32::MAX / 2, id.to_owned()));
                break;
            }
        }
        i += 1;
    }
    let rank = Rank(result);
    if left.is_some_and(|l| l >= &rank) || right.is_some_and(|r| &rank >= r) {
        return Err("Rank allocation escaped the interval".into());
    }
    Ok(rank)
}
