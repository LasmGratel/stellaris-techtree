use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use datasize::{data_size, DataSize};
use crate::Technology;

#[derive(Debug, Default, DataSize)]
pub struct TechnologyTree {
    pub start_tech: Vec<Rc<Node>>,
    pub dangling_tech: Vec<Rc<Node>>,

    pub node_map: HashMap<String, Rc<Node>>
}

impl TechnologyTree {
    pub fn insert_map(&mut self, tech_map: &HashMap<&str, Rc<Technology>>) {
        self.node_map = tech_map.iter().map(|(id, tech)| {
            (id.to_string(), Rc::new(Node {
                prev: RefCell::new(vec![]),
                next: RefCell::new(vec![]),

                name: id.to_string(),
                data: Some(tech.clone())
            }))
        }).collect();

        self.node_map.clone().into_iter().for_each(|(_, node)| {
            self.insert_node(node);
        });
    }

    pub fn insert_node(&mut self, node: Rc<Node>) {
        match node.data.as_ref().map(|x| x.prerequisites.clone()) {
            Some(prerequisites) => {
                prerequisites.into_iter().for_each(|prev| {
                    match self.node_map.get(prev.as_str()) {
                        Some(prev) => {
                            prev.next.borrow_mut().push(node.clone());
                            node.prev.borrow_mut().push(prev.clone());
                        }
                        None => {
                            let new_node = Rc::new(Node::from_name(prev.to_string()));
                            new_node.next.borrow_mut().push(node.clone());
                            node.prev.borrow_mut().push(new_node.clone());
                            self.insert_node(new_node.clone());
                        }
                    }
                });
            }
            None => {
                self.node_map.insert(node.name.to_string(), node.clone());
            }
        }
    }

    pub fn find(&self, root: Rc<Node>, tech: Rc<Technology>) -> Option<Rc<Node>> {
        if (root.data).as_ref().map_or(false, |x| x == &tech) {
            return Some(root);
        }

        return root.next.borrow().iter().find(|x| self.find((*x).clone(), tech.clone()).is_some()).cloned();
    }

    pub fn find_by_id(&self, root: Rc<Node>, id: &str) -> Option<Rc<Node>> {
        if root.name.as_str() == id {
            return Some(root);
        }

        return root.next.borrow().iter().find(|x| self.find_by_id((*x).clone(), id).is_some()).cloned();
    }
}

pub struct Node {
    pub(crate) prev: RefCell<Vec<Rc<Node>>>,
    pub(crate) next: RefCell<Vec<Rc<Node>>>,

    // should be unique
    pub name: String,

    // maybe unresolved
    pub data: Option<Rc<Technology>>
}

impl DataSize for Node {
    const IS_DYNAMIC: bool = true;
    const STATIC_HEAP_SIZE: usize = 0;

    fn estimate_heap_size(&self) -> usize {
        data_size::<Vec<Rc<Node>>>(self.prev.borrow().as_ref()) + data_size::<Vec<Rc<Node>>>(self.next.borrow().as_ref()) + data_size(&self.data) + data_size(&self.name)
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("name", &self.name)
            .field("next", &self.next.borrow().iter().map(|x| &x.name).collect::<Vec<&String>>())
            .field("prev", &self.prev.borrow().iter().map(|x| &x.name).collect::<Vec<&String>>())
            .finish()
    }
}

impl Node {
    pub fn from_name(name: String) -> Node {
        Node {
            prev: RefCell::new(vec![]),
            next: RefCell::new(vec![]),
            name,
            data: None
        }
    }
}