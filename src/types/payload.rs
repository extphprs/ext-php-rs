//! Borrow-tracked access to the Rust value of a class object.

use std::{
    cell::{Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
};

use crate::{exception::PhpException, zend::ce};

/// The reason the Rust value of a class object cannot be borrowed.
///
/// Each variant holds the PHP class name. The PHP exception made from it is
/// an `Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum BorrowError {
    /// A method of the object modifies the value.
    #[error("Cannot use object of class {0} while one of its methods modifies it")]
    MutablyBorrowed(&'static str),
    /// The value is in use, so it cannot be modified.
    #[error("Cannot modify object of class {0} while it is in use")]
    Borrowed(&'static str),
    /// The object has no Rust value, for example a subclass that does not
    /// call `parent::__construct()`.
    #[error("Object of class {0} is not initialized")]
    Uninitialized(&'static str),
}

impl From<BorrowError> for PhpException {
    fn from(err: BorrowError) -> Self {
        Self::new(err.to_string(), 0, ce::error())
    }
}

/// Shared access to the Rust value of a class object.
///
/// The value cannot be borrowed mutably while this guard lives.
#[derive(Debug)]
pub struct ClassRef<'a, T>(Ref<'a, T>);

/// Exclusive access to the Rust value of a class object.
///
/// The value cannot be borrowed again while this guard lives.
#[derive(Debug)]
pub struct ClassRefMut<'a, T>(RefMut<'a, T>);

impl<T> Deref for ClassRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> Deref for ClassRefMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for ClassRefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

pub(crate) fn borrow<'a, T>(
    cell: &'a RefCell<Option<T>>,
    class: &'static str,
) -> Result<ClassRef<'a, T>, BorrowError> {
    let value = cell
        .try_borrow()
        .map_err(|_| BorrowError::MutablyBorrowed(class))?;
    Ref::filter_map(value, Option::as_ref)
        .map(ClassRef)
        .map_err(|_| BorrowError::Uninitialized(class))
}

pub(crate) fn borrow_mut<'a, T>(
    cell: &'a RefCell<Option<T>>,
    class: &'static str,
) -> Result<ClassRefMut<'a, T>, BorrowError> {
    let value = cell
        .try_borrow_mut()
        .map_err(|_| BorrowError::Borrowed(class))?;
    RefMut::filter_map(value, Option::as_mut)
        .map(ClassRefMut)
        .map_err(|_| BorrowError::Uninitialized(class))
}

pub(crate) fn replace<T>(
    cell: &RefCell<Option<T>>,
    value: T,
    class: &'static str,
) -> Result<Option<T>, BorrowError> {
    let mut slot = cell
        .try_borrow_mut()
        .map_err(|_| BorrowError::Borrowed(class))?;
    Ok(slot.replace(value))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{BorrowError, borrow, borrow_mut, replace};

    #[test]
    fn shared_borrows_coexist_and_block_exclusive() {
        let cell = RefCell::new(Some(7_u32));
        let a = borrow(&cell, "Counter").expect("first shared borrow");
        let b = borrow(&cell, "Counter").expect("second shared borrow");
        assert_eq!(*a + *b, 14);
        assert_eq!(
            borrow_mut(&cell, "Counter").err(),
            Some(BorrowError::Borrowed("Counter"))
        );
    }

    #[test]
    fn exclusive_borrow_blocks_everything_until_dropped() {
        let cell = RefCell::new(Some(1_u32));
        {
            let mut value = borrow_mut(&cell, "Counter").expect("exclusive borrow");
            *value = 2;
            assert_eq!(
                borrow(&cell, "Counter").err(),
                Some(BorrowError::MutablyBorrowed("Counter"))
            );
        }
        assert_eq!(borrow(&cell, "Counter").map(|value| *value), Ok(2));
    }

    #[test]
    fn uninitialized_cell_refuses_borrows_until_replaced() {
        let cell = RefCell::new(None::<u32>);
        assert_eq!(
            borrow(&cell, "Counter").err(),
            Some(BorrowError::Uninitialized("Counter"))
        );
        assert_eq!(
            borrow_mut(&cell, "Counter").err(),
            Some(BorrowError::Uninitialized("Counter"))
        );
        assert_eq!(replace(&cell, 3, "Counter"), Ok(None));
        assert_eq!(borrow(&cell, "Counter").map(|value| *value), Ok(3));
    }

    #[test]
    fn replace_is_refused_while_borrowed() {
        let cell = RefCell::new(Some(1_u32));
        let _shared = borrow(&cell, "Counter");
        assert_eq!(
            replace(&cell, 2, "Counter"),
            Err(BorrowError::Borrowed("Counter"))
        );
    }

    #[test]
    fn messages_name_the_class() {
        assert_eq!(
            BorrowError::MutablyBorrowed("Counter").to_string(),
            "Cannot use object of class Counter while one of its methods modifies it"
        );
        assert_eq!(
            BorrowError::Borrowed("Counter").to_string(),
            "Cannot modify object of class Counter while it is in use"
        );
        assert_eq!(
            BorrowError::Uninitialized("Counter").to_string(),
            "Object of class Counter is not initialized"
        );
    }
}
