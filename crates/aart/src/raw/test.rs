use rand::{seq::SliceRandom, thread_rng, Rng};

use crate::Aart;

#[test]
fn basic_insert_str() {
    let mut tree = Aart::new();
    tree.insert("hello there", 1);
}

#[test]
fn map_test() {
    let mut tree = Aart::new();
    let mut res = Vec::new();
    for _ in 0..10000 {
        let a: u64 = thread_rng().gen();
        let b: u64 = thread_rng().gen();
        res.push((a, b));
        tree.insert(&a, b);
    }

    res.as_mut_slice().shuffle(&mut thread_rng());

    for (k, v) in res {
        assert_eq!(tree.get(&k), Some(&v));
    }
}

#[test]
fn basic_insert_pod() {
    let mut tree = Aart::new();
    tree.insert(&22u64, 23);
}

#[test]
fn seq_insert_pod() {
    let mut tree = Aart::new();
    for i in 0..100000u64 {
        tree.insert(&i, i);
    }
}
