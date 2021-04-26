/// Option extensions
pub trait OptionExt {
  /// Create an option value from self
  fn some(self) -> Option<Self>
  where
    Self: Sized;
  fn none(&self) -> Option<Self>
  where
    Self: Sized;
}

impl<T: Sized> OptionExt for T {
  fn some(self) -> Option<T> {
    Some(self)
  }

  fn none(&self) -> Option<T> {
    None
  }
}

#[test]
fn to_some_works_for_number() {
  assert_eq!(1.some(), Some(1));
  let a = 1;
  assert_eq!(a.some(), Some(1));
  assert_eq!(a, 1);
  assert_eq!(1.none(), None);
}

#[test]
fn to_some_works_for_tuple() {
  assert_eq!((1, 2).some(), Some((1, 2)));
  assert_eq!((1, 2).none(), None);
  assert_eq!((1, 2, 3).some(), Some((1, 2, 3)));
  assert_eq!((1, 2, 3).none(), None);
}
