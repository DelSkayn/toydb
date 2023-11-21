fn main() {
    let mut tree = dbg!(art::Art::<str, usize>::new());
    tree.display();
    tree.insert("hello world", 1);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    tree.display();
    tree.insert("hello moon ", 2);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));

    tree.insert("h", 3);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));

    tree.insert("hello foo", 4);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));

    tree.insert("hello boo", 5);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));

    tree.insert("hello voo", 6);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));
    assert_eq!(tree.get("hello voo").copied(), Some(6));

    tree.insert("hello voa", 7);
    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));
    assert_eq!(tree.get("hello voo").copied(), Some(6));

    dbg!(&tree);
    tree.display();
    assert_eq!(tree.get("hello world").copied(), Some(1));
    assert_eq!(tree.get("hello moon ").copied(), Some(2));
    assert_eq!(tree.get("h").copied(), Some(3));
    assert_eq!(tree.get("hello foo").copied(), Some(4));
    assert_eq!(tree.get("hello boo").copied(), Some(5));
    assert_eq!(tree.get("hello voo").copied(), Some(6));
    assert_eq!(tree.get("hello voa").copied(), Some(7));
}
