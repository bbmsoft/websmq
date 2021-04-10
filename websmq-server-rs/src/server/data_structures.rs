use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

#[derive(Debug, PartialEq, Eq)]
pub struct TopicTree<T>
where
    T: PartialEq + Eq + Hash,
{
    root: TreeNode<T>,
    size: usize,
}

impl<T> Default for TopicTree<T>
where
    T: PartialEq + Eq + Hash,
{
    fn default() -> Self {
        TopicTree::new()
    }
}

impl<T> TopicTree<T>
where
    T: PartialEq + Eq + Hash,
{
    pub fn new() -> TopicTree<T> {
        TopicTree {
            root: TreeNode::with_name(String::new()),
            size: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn insert_client(&mut self, path: &[String], client: T) -> bool {
        let clients = self.get_or_create_clients_at_path(&path);
        let inserted = clients.insert(client);
        if inserted {
            self.size += 1;
        }
        inserted
    }

    pub fn remove_client(&mut self, path: &[String], client: &T) -> bool {
        let removed = if let Some(clients) = self.get_clients_at_path(&path) {
            let removed = clients.remove(client);
            removed
        } else {
            false
        };
        self.cleanup(path);
        if removed {
            self.size -= 1;
        }
        removed
    }

    fn get_or_create_clients_at_path(&mut self, path: &[String]) -> &mut HashSet<T> {
        let mut current = &mut self.root;
        for element in path {
            current = current.get_or_create_child(element);
        }
        &mut current.clients
    }

    fn get_node_at_path(&mut self, path: &[String]) -> Option<&mut TreeNode<T>> {
        let mut current = Some(&mut self.root);
        for element in path {
            match current {
                Some(node) => current = node.get_child(element),
                None => return None,
            }
        }
        current
    }

    fn get_clients_at_path(&mut self, path: &[String]) -> Option<&mut HashSet<T>> {
        let node = self.get_node_at_path(path);
        node.map(|node| &mut node.clients)
    }

    fn cleanup(&mut self, path: &[String]) {
        if path.is_empty() {
            // can't clean up the root node
            return;
        }
        if let Some(node) = self.get_node_at_path(path) {
            if node.clients.is_empty() && node.children.is_empty() {
                self.remove_node(path);
            }
        }
    }

    fn remove_node(&mut self, path: &[String]) {
        if path.is_empty() {
            // can't remove the root node
            return;
        }

        let last = path.len() - 1;
        let parent_path = &path[..last];
        let node_name = &path[last];
        if let Some(parent) = self.get_node_at_path(parent_path) {
            parent.children.remove(node_name);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct TreeNode<T>
where
    T: PartialEq + Eq + Hash,
{
    children: HashMap<String, TreeNode<T>>,
    name: String,
    clients: HashSet<T>,
}

impl<T> TreeNode<T>
where
    T: PartialEq + Eq + Hash,
{
    fn with_name(name: String) -> TreeNode<T> {
        TreeNode {
            children: HashMap::new(),
            name,
            clients: HashSet::new(),
        }
    }

    fn get_child(&mut self, path_segment: &str) -> Option<&mut TreeNode<T>> {
        self.children.get_mut(path_segment)
    }

    fn get_or_create_child(&mut self, path_segment: &str) -> &mut TreeNode<T> {
        if self.children.contains_key(path_segment) {
            self.children
                .get_mut(path_segment)
                .expect("can't be none, we just checked")
        } else {
            let child = TreeNode::with_name(path_segment.to_owned());
            self.children.insert(path_segment.to_owned(), child);
            self.get_child(path_segment)
                .expect("cannot be None, we just inserted it")
        }
    }

    fn remove_child(&mut self, name: &str) -> Option<TreeNode<T>> {
        self.children.remove(name)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_insert() {
        let mut tree = TopicTree::new();
        tree.insert_client(
            &vec![
                "temps".to_owned(),
                "home".to_owned(),
                "kitchen".to_owned(),
                "fridge".to_owned(),
            ],
            "client-1",
        );
        assert_eq!(tree.len(), 1);
        tree.insert_client(
            &vec!["temps".to_owned(), "home".to_owned(), "kitchen".to_owned()],
            "client-1",
        );
        assert_eq!(tree.len(), 2);
        tree.insert_client(
            &vec!["temps".to_owned(), "home".to_owned(), "kitchen".to_owned()],
            "client-1",
        );
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut tree = TopicTree::new();
        tree.insert_client(
            &vec![
                "temps".to_owned(),
                "home".to_owned(),
                "kitchen".to_owned(),
                "fridge".to_owned(),
            ],
            "client-1",
        );
        tree.insert_client(
            &vec![
                "temps".to_owned(),
                "home".to_owned(),
                "kitchen".to_owned(),
                "fridge".to_owned(),
            ],
            "client-2",
        );
        tree.insert_client(
            &vec![
                "temps".to_owned(),
                "home".to_owned(),
                "kitchen".to_owned(),
                "fridge".to_owned(),
            ],
            "client-3",
        );
        tree.insert_client(
            &vec!["temps".to_owned(), "home".to_owned(), "kitchen".to_owned()],
            "client-1",
        );
        assert_eq!(tree.len(), 4);

        tree.remove_client(
            &vec!["temps".to_owned(), "home".to_owned(), "kitchen".to_owned()],
            &"client-1",
        );
        assert_eq!(tree.len(), 3);

        tree.remove_client(
            &vec![
                "temps".to_owned(),
                "home".to_owned(),
                "kitchen".to_owned(),
                "fridge".to_owned(),
            ],
            &"client-2",
        );
        assert_eq!(tree.len(), 2);

        let clients = tree
            .get_clients_at_path(&vec![
                "temps".to_owned(),
                "home".to_owned(),
                "kitchen".to_owned(),
                "fridge".to_owned(),
            ])
            .unwrap();

        assert!(clients.remove(&"client-1"));
        assert!(clients.remove(&"client-3"));
    }
}
