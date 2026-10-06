use std::sync::Arc;

#[derive(Debug)]
pub struct State<T>(pub Arc<T>);

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> std::ops::Deref for State<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> State<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(value))
    }
}

impl<T> From<T> for State<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> From<Arc<T>> for State<T> {
    fn from(arc: Arc<T>) -> Self {
        Self(arc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derefs_to_the_inner_value() {
        let state = State::new(7u32);

        assert_eq!(*state, 7);
    }

    #[test]
    fn clone_shares_the_same_allocation() {
        let state = State::new(String::from("shared"));
        let clone = state.clone();

        assert!(Arc::ptr_eq(&state.0, &clone.0));
    }

    #[test]
    fn converts_from_values_and_arcs() {
        let state: State<i64> = 42.into();
        assert_eq!(*state, 42);

        let arc = Arc::new(String::from("shared"));
        let state: State<String> = Arc::clone(&arc).into();
        assert!(Arc::ptr_eq(&arc, &state.0));
    }
}
