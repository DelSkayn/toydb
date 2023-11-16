fn main() {
    let mut tree = art::Art::<usize, usize>::new();
    tree.insert(&10, 1);
    assert_eq!(tree.get(10).copied(), Some(1));
}
