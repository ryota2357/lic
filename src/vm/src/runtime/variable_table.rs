use super::*;

#[derive(Default, Clone, Debug, PartialEq)]
pub struct VariableTable {
    scopes: Vec<internal::Scope>,
}

impl VariableTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![internal::Scope::new()],
        }
    }

    #[inline]
    pub fn push_scope(&mut self) {
        self.scopes.push(internal::Scope::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes
            .pop()
            .expect("[BUG] This should be called in at least one scope.");
    }

    pub fn push(&mut self, object: Object) {
        self.scopes
            .last_mut()
            .expect("[BUG] This should be called in at least one scope.")
            .push(internal::Entity::Value(object));
    }

    pub fn push_ref(&mut self, ref_object: Rc<RefCell<Object>>) {
        self.scopes
            .last_mut()
            .expect("[BUG] This should be called in at least one scope.")
            .push(internal::Entity::Shared(ref_object));
    }

    pub fn drop(&mut self, count: usize) {
        self.scopes
            .last_mut()
            .expect("[BUG] This should be called in at least one scope.")
            .drop(count);
    }

    pub fn edit(&mut self, id: LocalId, object: Object) {
        self.scopes
            .last_mut()
            .expect("[BUG] This should be called in at least one scope.")
            .edit(id, object)
    }

    pub fn get(&self, id: LocalId) -> Object {
        self.scopes
            .last()
            .expect("[BUG] This should be called in at least one scope.")
            .get(id)
    }

    pub fn get_ref(&mut self, id: LocalId) -> Rc<RefCell<Object>> {
        self.scopes
            .last_mut()
            .expect("[BUG] This should be called in at least one scope.")
            .get_ref(id)
    }

    pub fn dump(&self, indent: usize) {
        println!("{}[VariableTable]", " ".repeat(indent));
        for scope in self.scopes.iter() {
            scope.dump(indent + 2);
        }
    }
}

mod internal {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    pub enum Entity {
        Value(Object),
        Shared(Rc<RefCell<Object>>),
    }

    #[derive(Default, Clone, Debug, PartialEq)]
    pub struct Scope {
        entities: Vec<Entity>,
    }

    impl Scope {
        #[inline]
        pub const fn new() -> Self {
            Self {
                entities: Vec::new(),
            }
        }

        #[inline]
        pub fn push(&mut self, entity: Entity) {
            self.entities.push(entity);
        }

        pub fn drop(&mut self, count: usize) {
            if count > self.entities.len() {
                panic!(
                    "[BUG] Cannot drop {} variables because there are only {} variables in scope.",
                    count,
                    self.entities.len()
                );
            }
            self.entities.truncate(self.entities.len() - count);
        }

        pub fn get(&self, id: LocalId) -> Object {
            if let Some(entity) = self.entities.get(id.0) {
                match entity {
                    Entity::Value(object) => object.clone(),
                    Entity::Shared(object) => object.borrow().clone(),
                }
            } else {
                panic!(
                    "[BUG] LocalId out of range. Expected 0..{}, but got {}.",
                    self.entities.len(),
                    id.0
                );
            }
        }

        pub fn get_ref(&mut self, id: LocalId) -> Rc<RefCell<Object>> {
            if let Some(entity) = self.entities.get(id.0) {
                match entity {
                    Entity::Value(object) => {
                        let res = Rc::new(RefCell::new(object.clone()));
                        self.entities[id.0] = Entity::Shared(Rc::clone(&res));
                        res
                    }
                    Entity::Shared(object) => Rc::clone(object),
                }
            } else {
                panic!(
                    "[BUG] LocalId out of range. Expected 0..{}, but got {}.",
                    self.entities.len(),
                    id.0
                );
            }
        }

        pub fn edit(&mut self, id: LocalId, object: Object) {
            if let Some(entity) = self.entities.get(id.0) {
                match entity {
                    Entity::Value(_) => {
                        self.entities[id.0] = Entity::Value(object);
                    }
                    Entity::Shared(entity) => {
                        *(entity.borrow_mut()) = object;
                    }
                }
            } else {
                panic!(
                    "[BUG] LocalId out of range. Expected 0..{}, but got {}.",
                    self.entities.len(),
                    id.0
                );
            }
        }

        pub fn dump(&self, indent: usize) {
            println!("{}[Scope]", " ".repeat(indent));
            for (idx, entity) in self.entities.iter().enumerate() {
                println!("{}{idx}: {entity:?}", " ".repeat(indent + 2));
            }
        }
    }
}
