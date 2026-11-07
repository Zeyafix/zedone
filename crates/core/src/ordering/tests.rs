use super::*;
fn id(n: u32) -> String {
    format!("{n:032x}")
}

#[test]
fn dense_paths_insert_repeatedly_at_both_boundaries_and_between_tied_legacy_ranks() {
    let mut ranks = vec![
        Rank::Legacy(0).key(&id(1)),
        Rank::Legacy(0).key(&id(2)),
        Rank::Legacy(u64::MAX).key(&id(3)),
    ];
    for n in 4..604 {
        let i = match n % 3 {
            0 => 0,
            1 => ranks.len(),
            _ => 1,
        };
        let left = i.checked_sub(1).map(|i| ranks[i].as_slice());
        let right = ranks.get(i).map(Vec::as_slice);
        let rank = Rank::between(left, right, &id(n)).unwrap();
        assert!(rank.valid());
        let key = rank.key(&id(n));
        assert!(left.is_none_or(|l| l < key.as_slice()));
        assert!(right.is_none_or(|r| key.as_slice() < r));
        ranks.insert(i, key);
    }
    assert!(ranks.windows(2).all(|v| v[0] < v[1]));
    assert!(Rank::between(Some(&ranks[2]), Some(&ranks[1]), &id(9)).is_err());
}

#[test]
fn independent_objects_in_one_gap_sort_stably_and_their_gap_stays_dense() {
    let left = Rank::Legacy(0).key(&id(1));
    let right = Rank::Legacy(1).key(&id(2));
    let a = Rank::between(Some(&left), Some(&right), &id(3))
        .unwrap()
        .key(&id(3));
    let b = Rank::between(Some(&left), Some(&right), &id(4))
        .unwrap()
        .key(&id(4));
    assert!(a < b);
    let c = Rank::between(Some(&a), Some(&b), &id(5))
        .unwrap()
        .key(&id(5));
    assert!(a < c && c < b);
}

#[test]
fn long_append_sequences_reuse_spare_digits_instead_of_growing_per_object() {
    let mut left = Rank::Legacy(0).key(&id(1));
    for n in 2..10_002 {
        let next = Rank::between(Some(&left), None, &id(n))
            .unwrap()
            .key(&id(n));
        assert!(left < next);
        assert!(next.len() <= 2);
        left = next;
    }
    let upper = Rank::Legacy(1).key(&id(2));
    let mut left = Rank::Legacy(0).key(&id(1));
    for n in 3..10_003 {
        let next = Rank::between(Some(&left), Some(&upper), &id(n))
            .unwrap()
            .key(&id(n));
        assert!(left < next && next < upper);
        assert!(next.len() <= 3);
        left = next;
    }
}
