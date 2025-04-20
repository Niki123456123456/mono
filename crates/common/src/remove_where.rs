pub trait RemoveWhere<T> {
    fn remove_where(&mut self, filter: impl FnMut(&mut T) -> bool);
}

impl<T> RemoveWhere<T> for Vec<T> {
    fn remove_where(&mut self, mut filter: impl FnMut(&mut T) -> bool) {
        if self.len() > 0 {
            let mut i = self.len() - 1;
            loop {
                if (filter)(&mut self[i]) {
                    self.remove(i);
                }
                if i == 0 {
                    break;
                }
                i -= 1;
            }
        }
    }
}
